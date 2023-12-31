# Cohost Random Workshop Item Bot

Posts random workshop items to [Cohost](https://cohost.org/l4d2workshop) using [reqwest](https://github.com/seanmonstar/reqwest) and [eggbug-rs](https://github.com/iliana/eggbug-rs).

Requires .env file with the following fields filled in
```
COHOST_EMAIL=your_email
COHOST_PASSWORD=your_password
COHOST_PROJECT=page_name_to_post_to (omit the @)
APP_ID=number (steam app ID, eg 440 for TF2)
```
Note that
1. Name in Cargo.toml must match folder name. Caused by CARGO_MANIFEST_DIR env var. If anyone feels like fixing this, please submit a PR.
1. Limited to the top or most recent 50,000 items.
2. Games with less than 50,000 workshop items are unsupported. I'll happily take PRs that address this.
3. Reliant on scraping, so if Valve changes the workshop html, it might break.
