# Cohost Random Workshop Item Bot

Posts random workshop items to [Cohost](https://cohost.org/l4d2workshop) using [reqwest](https://github.com/seanmonstar/reqwest) and [eggbug-rs](https://github.com/iliana/eggbug-rs).

Requires .env file with the following fields filled in
```
COHOST_EMAIL=your_email
COHOST_PASSWORD=your_password
COHOST_PROJECT=page_name_to_post_to (omit @)
APP_ID=number (steam app ID, eg 440 for TF2)
```
