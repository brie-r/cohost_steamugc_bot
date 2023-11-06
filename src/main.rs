#![deny(elided_lifetimes_in_paths)]
#![warn(clippy::pedantic)]

use anyhow::Result;
use eggbug::{Attachment, Client, Post};
use std::path::Path;
use tracing_subscriber::{fmt, EnvFilter};
use rand::{Rng, thread_rng, distributions::Alphanumeric};
use std::{fs::File, io::{copy, Cursor}};
use html_escape;

#[tokio::main]
async fn main() -> Result<()> {
	dotenv::dotenv().ok();
	fmt().with_env_filter(EnvFilter::from_default_env()).init();

	let items_per_page = 30;

	let workshop_url_start = r"https://steamcommunity.com/workshop/browse/?appid=550&browsesort=trend&section=readytouseitems&actualsort=trend&p=";
	let workshop_url_end = r"&days=-1&numperpage=30";

	let mut rng = rand::thread_rng();
	let random_int = rng.gen_range(0..50010);
	let num_page = random_int / items_per_page;
	let num_offset = random_int % items_per_page;

	let page_url = workshop_url_start.to_owned() + &num_page.to_string() + workshop_url_end;
	let page = reqwest::get(page_url)
		.await?
		.text()
		.await?;

	let search_start_url = "https";
	let search_start_item = "class=\"workshopItem\">";
	let search_end_item = "&";

	let mut start_index = page.nth_index_of(search_start_item, 0, num_offset);
	start_index = page.index_of(search_start_url, start_index.unwrap());
	let mut end_index = page.index_of(search_end_item, start_index.unwrap());
	let item_url = &page[start_index.unwrap()..end_index.unwrap()];
	println!("{}", item_url);

	let search_start_image = "workshopItemPreviewImage";
	let search_end_image = "?";

	start_index = page.index_of(search_start_image, end_index.unwrap());
	start_index = page.index_of(search_start_url, start_index.unwrap());
	end_index = page.index_of(search_end_image, start_index.unwrap());
	let image_url = &page[start_index.unwrap()..end_index.unwrap()];
	println!("{}", image_url);

	let search_start_title = "workshopItemTitle ellipsis";
	let search_end_title = "</div>";
	start_index = page.index_of(search_start_title, end_index.unwrap());
	let start_index_modified = start_index.unwrap() + search_start_title.len() + 2;
	end_index = page.index_of(search_end_title, start_index_modified);
	let title = &page[start_index_modified..end_index.unwrap()];
	let title_decoded = html_escape::decode_html_entities(title);
	println!("{}", title_decoded);

	let image_file_name: String = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(12)
		.map(char::from)
		.collect();

	match download_image_to(image_url, &(image_file_name.to_owned() + ".png")).await
	{
		Ok(_) => println!("{}", image_file_name.to_owned() + ".png downloaded"),
		Err(e) => println!("error while downloading image: {}", e),
	}

	let email = std::env::var("COHOST_EMAIL")?;
	let password = std::env::var("COHOST_PASSWORD")?;
	let project = std::env::var("COHOST_PROJECT")?;

	let client = Client::new();
	let session = client.login(&email, &password).await?;

	let mut post = Post
	{
		headline: "L4D2 Workshop Item of the Day: ".to_string() + &title_decoded,
		attachments: vec!
		[
			Attachment::new_from_file
			(
				Path::new(env!("CARGO_MANIFEST_DIR"))
					.join(&(image_file_name.to_owned() + ".png")),
				"image/png".into(),
				None,
			)
			.await?
		],
		tags: vec!["🤖".to_string(), "bot".to_string(), "The Cohost Bot Feed".to_string(), "Steam Workshop Bot".to_string()],
		draft: false,
		..Default::default()
	};
	let id = session.create_post(&project, &mut post).await?;

	post.markdown = "[".to_string() + &title_decoded + "](" + item_url + ")";
	session.edit_post(&project, id, &mut post).await?;
	
	match std::fs::remove_file(&(image_file_name.to_owned() + ".png"))
	{
		Ok(_) => println!("{}", image_file_name.to_owned() + ".png deleted"),
		Err(e) => println!("error while deleting image: {}", e),
	}

	Ok(())
}

async fn index_of(haystack: &str, needle: &str, start_index: usize) -> Option<usize>
{
	if start_index >= haystack.len()
	{
		return None;
	}

	let substring = &haystack[start_index..];

	match substring.find(needle)
	{
		Some(index) => Some(start_index + index),  // if found, return the adjusted index
		None => None, // if not found, return None
	}
}

trait IndexOf {
	fn index_of(&self, needle: &str, start_index: usize) -> Option<usize>;
	fn nth_index_of(&self, needle: &str, start_index: usize, instance: usize) -> Option<usize>;
}

impl IndexOf for str
{
	fn index_of(&self, needle: &str, start_index: usize) -> Option<usize> {
		if start_index >= self.len() {
			return None;
		}
		let substring = &self[start_index..];
		match substring.find(needle) {
			Some(index) => Some(start_index + index),
			None => None,
		}
	}
	fn nth_index_of(&self, needle: &str, start_index: usize, instance: usize) -> Option<usize> {
		if start_index >= self.len() {
			return None;
		}
		let mut substring = &self[start_index..];
		let mut total_index = start_index;
		for _ in 0..instance {
			if let Some(index) = substring.find(needle) {
				total_index += index + needle.len();
				substring = &self[total_index..];
			} else {
				return None;
			}
		}
		Some(total_index - needle.len())
	}	
}
async fn download_image_to(url: &str, file_name: &str) -> Result<()>
{
	// Send an HTTP GET request to the URL
	let response = reqwest::get(url).await?;
	// Create a new file to write the downloaded image to
	let mut file = File::create(file_name)?;
	
	// Create a cursor that wraps the response body
	let mut content =  Cursor::new(response.bytes().await?);
	// Copy the content from the cursor to the file
	copy(&mut content, &mut file)?;
	Ok(())
}