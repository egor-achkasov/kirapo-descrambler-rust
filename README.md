# COMIC Meteor (COMICメテオ, kirapo.jp/meteor) manga downloader and descrambler

Downloads comics from https://kirapo.jp/meteor and descrables the obfuscated pages.

## Building from source and usage

- Download or clone the repository
- Install [Rust stuff](rustup.rs)
- Open the repo in terminal/powershell and run `cargo run *url to your comic*`. The URL should end with `/viewer`
Example: `cargo run https://kirapo.jp/pt/meteor/jyashin/1000219/viewer`

In case of success this will create a directory with the panels in the repository.