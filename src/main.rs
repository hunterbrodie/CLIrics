extern crate mpris;

use mpris::{Metadata, Player, PlayerFinder};

use lyricrustacean::get_lyrics;

use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute, queue,
    style::{self, SetAttribute, Attribute},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    event::{poll, read}
};

use std::{
    error::Error,
    io::{stdout, Write},
    thread,
    sync::mpsc::{self, Sender, Receiver},
    time::Duration
};

struct Data {
    artist: Option<String>,
    title: Option<String>,
    lyrics: Option<Vec<String>>,
    scroll: Option<Scroll>,
    exit: bool
}

enum Scroll {
    Up,
    Down,
    Reset
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    execute!(
        stdout(),
        cursor::Hide,
        terminal::Clear(ClearType::All),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;

    let (mpris_tx, rx): (Sender<Data>, Receiver<Data>) = mpsc::channel();
    let input_tx = mpris_tx.clone();

    thread::spawn(move || {
        mpris_listen(mpris_tx);
    });

    thread::spawn(move || {
        input_listen(input_tx);
    });

    let mut artist = String::new();
    let mut title = String::new();
    let mut lyrics: Vec<String> = Vec::new();
    let mut start: usize = 0;

    for received in rx {
        if received.exit {
            break;
        }

        match received.artist {
            Some(e) => artist = e,
            None => ()
        }
        match received.title {
            Some(e) => title = e,
            None => ()
        }
        match received.lyrics {
            Some(e) => lyrics = e,
            None => ()
        }
        match received.scroll {
            Some(e) => match e {
                Scroll::Up => {
                    /*if !(start >= lyrics.len()) {
                        start += 1;
                    }*/
                    if (lyrics.len() - start) as u16 > terminal::size().expect("Coudln't get terminal size").1 - 2 {
                        start += 1;
                    }
                },
                Scroll::Down => {
                    if start != 0 {
                        start -= 1;
                    }
                },
                Scroll::Reset => start = 0,
            },
            None => ()
        }
        
        print_lyrics(&artist, &title, &lyrics, &start)?;

        //let mut stdout = stdout();
    }

    execute!(
        stdout(),
        cursor::Show,
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    disable_raw_mode()?;

    Ok(())
}

fn input_listen(tx: Sender<Data>) {
    loop {
        if poll(Duration::from_millis(100)).expect("Couldn't read input") {
            let mut scroll: Option<Scroll> = None;
            let mut exit = false;

            let event = read().expect("Failed to read input");
            match event {
                crossterm::event::Event::Key(e) => {
                    if e.modifiers.bits() == 0b0000_0010 && match e.code {
                        crossterm::event::KeyCode::Char(c) => c.eq(&'c'),
                        _ => false
                    } {
                        exit = true;
                    }
                },
                crossterm::event::Event::Mouse(e) => {
                    match e.kind {
                        crossterm::event::MouseEventKind::ScrollDown => scroll = Some(Scroll::Up),
                        crossterm::event::MouseEventKind::ScrollUp => scroll = Some(Scroll::Down),
                        _ => ()
                    }
                },
                _ => ()
            }

            tx.send(Data{
                artist: None,
                title: None,
                lyrics: None,
                scroll: scroll,
                exit: exit
            }).expect("Failed to send data");
        }
    }
}

fn mpris_listen(tx: Sender<Data>) {
    let player = match get_cmus() {
        Some(p) => p,
        None => panic!("Could not find CMUS player"),
    };
    let tuple = get_metadata(player.get_metadata().expect("failed to get metadata"));

    tx.send(Data {
        artist: Some(tuple.0),
        title: Some(tuple.1),
        lyrics: Some(tuple.2),
        scroll: Some(Scroll::Reset),
        exit: false
    }).expect("Failed to send data");

    let events = player.events().expect("Could not start event stream");

    for event in events {
        match event {
            Ok(event) => match event {
                mpris::Event::TrackChanged(e) => {
                    let tuple = get_metadata(e);

                    tx.send(Data {
                        artist: Some(tuple.0),
                        title: Some(tuple.1),
                        lyrics: Some(tuple.2),
                        scroll: Some(Scroll::Reset),
                        exit: false
                    }).expect("Failed to send data");
                }
                _ => continue,
            },
            Err(err) => {
                println!("D-Bus error: {}. Aborting.", err);
                break;
            }
        }
    }
}

fn get_metadata(metadata: Metadata) -> (String, String, Vec<String>) {
    let mut tuple: (String, String, Vec<String>) = (String::new(), String::new(), Vec::new());

    match metadata.artists() {
        Some(e) => tuple.0 = e[0].clone(),
        None => (),
    };

    match metadata.title() {
        Some(e) => tuple.1 = e.clone().to_owned(),
        None => (),
    }

    match get_lyrics(&tuple.0, &tuple.1) {
        Some(e) => tuple.2 = e,
        None => tuple.2 = vec!["Can't Find Lyrics".to_owned()],
    };

    tuple
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

fn print_lyrics(artist: &str, title: &str, lyrics: &Vec<String>, start: &usize) -> Result<(), Box<dyn Error>> {
    let height = terminal::size()?.1 as usize;

    let mut stdout = stdout();
    queue!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        style::Print(" "),
        style::SetAttribute(Attribute::Bold),
        style::SetAttribute(Attribute::Underlined),
        style::Print(format!("{} - {}", &artist, &title)),
        SetAttribute(Attribute::Reset),
        cursor::MoveTo(0, 1)
    )?;

    for i in 1..height {
        let index = start + i - 1;
        if i > height - 2 || index >= lyrics.len() {
            break;
        }
        queue!(
            stdout,
            cursor::MoveTo(0, (i + 1) as u16),
            style::Print(format!(" {}", &lyrics[index]))
        )?;
    }

    stdout.flush()?;

    Ok(())
}
