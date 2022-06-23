extern crate mpris;

use mpris::PlayerFinder;
use mpris::Player;
use mpris::Event;

// Pauses currently playing media and prints metadata information about that
// media.
// If no player is running, exits with an error.
fn main() {
    //player.pause().expect("Could not pause");

    //let metadata = player.get_metadata().expect("Could not get metadata for player");
    //println!("{:#?}", metadata);

    let player = match get_cmus() {
        Some(p) => p,
        None => panic!("Could not find CMUS player")
    };

    let events = player.events().expect("Could not start event stream");

    for event in events {
        match event {
            Ok(event) => get_lyrics(event),
            Err(err) => {
                println!("D-Bus error: {}. Aborting.", err);
                break;
            }
        }
    }
}

fn get_lyrics(event: Event) {

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