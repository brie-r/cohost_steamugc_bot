﻿#![deny(elided_lifetimes_in_paths) ]
#![warn(clippy::pedantic) ]

use anyhow::Result;
use eggbug::{Attachment, Client, Post};
use tracing_subscriber::{fmt, EnvFilter};
use rand::{Rng, thread_rng, distributions::Alphanumeric};
use std::{path::Path, fs::File, io::{copy, Cursor}};
use html_escape::decode_html_entities;
//use string_search::{StringSearch, Include};

#[tokio::main]
async fn main() -> Result<()> {
	dotenv::dotenv().ok();
	fmt().with_env_filter ( EnvFilter::from_default_env() ).init();

	let workshop_url_start = r"https://steamcommunity.com/workshop/browse/?appid=";
	let app_id = std::env::var( "APP_ID" )?;
	let workshop_url_mid = r"&browsesort=trend&section=readytouseitems&actualsort=trend&p=";
	let workshop_url_end = r"&days=-1&numperpage=";

	// Set to 50000 because workshop displays only the rest of the page past the 50000th item.
	// For workshops smaller than 50000 items, choose an approach:
	//	1. Adjust the number manually
	//	2. Write code that dynamically determines the max
	//		Max displayed in <div class="workshopBrowsePagingInfo">
	//		Note that the last page will cause issues when we roll random_item_index
	let workshop_max_items = 50000;

	// Workshop limited to 9, 18, or 30 items per page.
	// Choosing 9 because it means less html to search.
	let items_per_page = 9;
	let mut rng = rand::thread_rng();
	let max_page = workshop_max_items / items_per_page + 1;

	// Inclusive range, 1-indexed
	// Page 0 is identical to page 1.
	// For 50000 items, 9 per page, 50000/9+1 = page 5556, which returns results. Page 5557 does not.
	let random_page_index = rng.gen_range(1 ..= max_page);
	// Inclusive range, 1-indexed
	let random_item_index = rng.gen_range(1 ..= items_per_page);

	let page_url =
		workshop_url_start.to_owned() +
		&app_id.to_string() +
		workshop_url_mid +
		&random_page_index.to_string() +
		workshop_url_end +
		&items_per_page.to_string();

	let page = reqwest::get(page_url)
		.await?
		.text()
		.await?;

	// Code order matches steam's html order; we only increase start_index. Could be made more flexible later, but it works.

	// Find nth item's parent <div> by class
	let search_start_item = "class=\"workshopItem\">";
	let start_index = page.nth_index_of ( 0, search_start_item, random_item_index );
	
	// Find target item's workshop url
	// Find next url, which will be in an <a href>
	// End url before &, discarding unnecessary query string
	let item_url = page.substring_find(start_index.unwrap(), &vec!["https"], EndPos::Start, &vec!["&"], EndPos::Start);
	println!( "{}", item_url.unwrap() );

	// Find target item's preview image url
	// Find url by class, which will be in an <img>
	// End url before &, discarding unnecessary query string
	let image_url = page.substring_find(start_index.unwrap(), &vec!["workshopItemPreviewImage", "https"], EndPos::Start, &vec!["?"], EndPos::Start);
	println!( "{}", image_url.unwrap() );

	// Find target item's title
	// The only good signpost for the title is the div, so we find it, and offset the start index to after the div string
	let title_decoded = decode_html_entities ( page.substring_find(start_index.unwrap(), &vec!["<div class=\"workshopItemTitle ellipsis\">"], EndPos::End, &vec!("</div>"), EndPos::Start).unwrap() );
	println!( "{}", title_decoded );

	let image_file_str: String = thread_rng ()
		.sample_iter ( &Alphanumeric )
		.take ( 12 )
		.map ( char::from )
		.collect ();

	let image_file_name = image_file_str + ".png";
	match download_from_url ( image_url.unwrap(), &image_file_name ).await
	{
		Ok(()) => println! ( "{image_file_name} downloaded" ),
		Err(e) => println! ( "error downloading {image_file_name}: {e}" ),
	}

	let email = std::env::var( "COHOST_EMAIL" )?;
	let password = std::env::var( "COHOST_PASSWORD" )?;
	let project = std::env::var( "COHOST_PROJECT" )?;

	let client = Client::new();
	let session = client.login ( &email, &password ).await?;

	let mut post = Post
	{
		headline: "L4D2 Workshop Item of the Day: ".to_string() + &title_decoded,
		attachments: vec!
		[
			Attachment::new_from_file
			(
				Path::new ( env! ( "CARGO_MANIFEST_DIR" ) )
					.join ( &image_file_name ),
				"image/png".into(),
				None,
			)
			.await?
		],
		tags: vec! [ "🤖".to_string(), "bot".to_string(), "The Cohost Bot Feed".to_string(), "Steam Workshop Bot".to_string() ],
		draft: false,
		 .. Default::default()
	};
	let id = session.create_post( &project, &mut post ).await?;

	post.markdown = "[".to_string() + &title_decoded + "]( " + item_url.unwrap() + " )";
	session.edit_post ( &project, id, &mut post ).await?;
	
	match std::fs::remove_file ( &image_file_name )
	{
		Ok(()) => println!( "{image_file_name} deleted" ),
		Err(e) => println!( "error deleting {image_file_name}: {e}" ),
	}

	Ok(())
}

enum EndPos { Start, End, }

trait StringSearch
{
	fn index_of ( &self, start_index: usize, find: &str ) -> Option <usize>;
	fn nth_index_of ( &self, start_index: usize, find: &str, instance: usize ) -> Option <usize>;
	fn index_of_multi ( &self, start_index: usize, find: &Vec<&str>, result_pos: EndPos ) -> Option <usize>;
	fn substring_find ( &self, start_index: usize, start: &Vec<&str>, result_pos_start: EndPos, end: &Vec<&str>, result_pos_end: EndPos ) -> Option <&str>;
}

impl StringSearch for str
{
	fn index_of ( &self, start_index: usize, find: &str ) -> Option < usize>
	{
		if start_index >= self.len()
		{
			return None;
		}
		let substring = &self [ start_index .. ];
		match substring.find (find)
		{
			Some ( index ) => Some ( start_index + index ),
			None => None,
		}
	}
	fn index_of_multi( &self, mut start_index: usize, find: &Vec<&str>, result_pos: EndPos ) -> Option <usize>
	{
		if start_index >= self.len()
		{
			return None;
		}

		let mut test: Option <usize> = None;
		for &item in find
		{
			let substring = &self [ start_index .. ];
			test = match substring.find ( item )
			{
				Some ( index ) =>
				{
					start_index += index;
					if matches!( result_pos, EndPos::End )
					{
						start_index += item.len();
					}
					Some ( start_index )
				},
				None => return None,
			};
			//println!("{:?}", test);
		}
		test
	}
	// Instance is 1-indexed.
	fn nth_index_of ( &self, start_index: usize, find: &str, instance: usize ) -> Option <usize>
	{
		if start_index >= self.len()
		{
			return None;
		}
		let mut substring = &self [ start_index .. ];
		let mut total_index = start_index;
		for _ in 0 .. instance
		{
			if let Some ( index ) = substring.find ( find )
			{
				total_index += index + find.len();
				substring = &self [ total_index .. ];
			}
			else
			{
				return None;
			}
		}
		Some ( total_index - find.len() )
	}
	fn substring_find ( &self, start_index: usize, start_find: &Vec<&str>, result_pos_start: EndPos, end_find: &Vec<&str>, result_pos_end: EndPos ) -> Option <&str>
	{
		let start = match self.index_of_multi(start_index, start_find, result_pos_start)
		{
			Some (usize) => Some (usize),
			None => return None,
		};
		let end = match self.index_of_multi ( start.unwrap(), end_find, result_pos_end )
		{
			Some (usize) => Some (usize),
			None => return None,
		};
		Some ( &self [ start.unwrap() .. end.unwrap() ] )
	}
}
async fn download_from_url( url: &str, file_name: &str ) -> Result<()>
{
	// Credit to Thorsten Hans at https://www.thorsten-hans.com/weekly-rust-trivia-download-an-image-to-a-file/
	let response = reqwest::get ( url ).await?;
	let mut file = File::create ( file_name )?;
	let mut content = Cursor::new ( response.bytes().await? );
	copy ( &mut content, &mut file )?;
	Ok(())
}