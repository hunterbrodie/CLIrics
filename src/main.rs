extern crate mpris;

use mpris::PlayerFinder;
use mpris::Player;
use mpris::Event;

use scraper::Html;
use scraper::Selector;
use scraper::ElementRef;

use std::io::{self, Write};

// Pauses currently playing media and prints metadata information about that
// media.
// If no player is running, exits with an error.
fn main() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();

    let player = match get_cmus() {
        Some(p) => p,
        None => panic!("Could not find CMUS player")
    };

    listen(player);

    /*match get_lyrics("JPEGMAFIA".to_owned(), "I used to be into dope".to_owned()) {
        Some(e) => print_lyrics(&e),
        None => print_lyrics("Can't find lyrics")
    };*/
}

fn listen(player: Player) {
    let events = player.events().expect("Could not start event stream");

    for event in events {
        match event {
            Ok(event) => match event {
                Event::TrackChanged(e) => {
                    let artist = e.artists().unwrap()[0].to_owned();
                    let title = e.title().unwrap().to_owned();

                    let lyrics = match get_lyrics(artist, title) {
                        Some(e) => e,
                        None => continue
                    };
                    print_lyrics(&lyrics);
                },
                _ => continue
            },
            Err(err) => {
                println!("D-Bus error: {}. Aborting.", err);
                break;
            }
        }
    }
}

fn print_lyrics(msg: &str) {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
    println!("{}", msg);
}

fn get_lyrics(artist: String, song: String) -> Option<String> {
    let client = reqwest::blocking::Client::new();
    let url = format!("https://www.azlyrics.com/lyrics/{}/{}.html", artist.to_lowercase().replace(" ", ""), song.to_lowercase().replace(" ", ""));
    let resp = client.get(&url)
        .send()
        .unwrap()
        .text()
        .unwrap();
    
    let document = Html::parse_document(resp.as_str());
    let body_selector = Selector::parse("body").unwrap();

    let body = document.select(&body_selector).next().unwrap();

    let div = match find_div_child(&body, "container main-page") {
        Some(e) => e,
        None => return None
    };
    let div = match find_div_child(&div, "row") {
        Some(e) => e,
        None => return None
    };
    let div = match find_div_child(&div, "col-xs-12 col-lg-8 text-center") {
        Some(e) => e,
        None => return None
    };

    let div_selector = Selector::parse("div").unwrap();

    for element in div.select(&div_selector) {
        match element.value().attr("class") {
            Some(_) => continue,
            None => {
                let mut lyrics = String::new();
                
                for line in element.text().collect::<Vec<_>>() {
                    let line = line.to_owned().to_owned();

                    if !line.is_empty() && !line.contains("freestar.config") {
                        line.trim().to_owned().push_str("\n");
                        lyrics.push_str(&line);
                    }
                }

                return Some(lyrics.trim().to_owned());
            }
        }
    }

    return None;
}

fn find_div_child<'a>(fragment: &'a ElementRef, class: &str) -> Option<ElementRef<'a>> {
    let div_selector = Selector::parse("div").unwrap();

    for element in fragment.select(&div_selector) {
        match element.value().attr("class") {
            Some(a) => {
                if a.eq(class) {
                    return Some(element);
                }
            },
            None => continue
        }
    }

    return None;
}

fn get_cmus() -> Option<Player<'static>> {
    let player_finder = PlayerFinder::new().expect("Could not connect to D-Bus");

    let all_players = player_finder.find_all().expect("Can't find players");
    
    for player in all_players {
        if format!("{}", player.bus_name()).ends_with("cmus") {
            return Some(player);
        }
    }

    return None;
}