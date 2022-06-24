extern crate mpris;

use mpris::PlayerFinder;
use mpris::Player;
use mpris::Event;
use mpris::Metadata;

use scraper::Html;
use scraper::Selector;
use scraper::ElementRef;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}
};
use std::{
    error::Error,
    io
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};


fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture,)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    listen(&mut terminal)?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn draw_frame<B: Backend>(f: &mut Frame<B>, artist: &str, song: &str, lyrics: Vec<String>) {
    let size = f.size();
    
    let block = Block::default().style(Style::default());
    f.render_widget(block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Percentage(100),
            ]
            .as_ref(),
        )
        .split(size);

    let text: Vec<Spans> = lyrics.into_iter().map(|l| Spans::from(l)).collect();

    let create_block = |title| {
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default())
            .title(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            ))
    };

    let paragraph = Paragraph::new(text)
        .style(Style::default())
        .block(create_block(format!("{} - {}", artist, song)))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, chunks[0]);
}

fn listen<B: Backend>(terminal: &mut Terminal<B>) -> Result<(), Box<dyn Error>> {
    let player = match get_cmus() {
        Some(p) => p,
        None => panic!("Could not find CMUS player")
    };
    
    display_metadata(terminal, player.get_metadata().expect("Could not get initial metadata"))?;

    let events = player.events().expect("Could not start event stream");

    for event in events {
        match event {
            Ok(event) => match event {
                Event::TrackChanged(e) =>  display_metadata(terminal, e)?,
                _ => continue
            },
            Err(err) => {
                println!("D-Bus error: {}. Aborting.", err);
                break;
            }
        }
    }

    Ok(())
}

fn display_metadata<B: Backend>(terminal: &mut Terminal<B>, metadata: Metadata) -> Result<(), Box<dyn Error>> {
    let artist = metadata.artists().unwrap()[0].clone();
    let title = metadata.title().unwrap();

    match get_lyrics(&artist, &title) {
        Some(e) => terminal.draw(|f| draw_frame(f, &artist, &title, e))?,
        None => terminal.draw(|f| draw_frame(f, &artist, &title, vec!["Can't Find Lyrics".to_owned()]))?
    };

    Ok(())
}

fn get_lyrics(artist: &str, song: &str) -> Option<Vec<String>> {
    let client = reqwest::blocking::Client::new();
    let url = format!("https://www.azlyrics.com/lyrics/{}/{}.html", format_az_metadata(artist), format_az_metadata(song));
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
                let mut lyrics: Vec<String> = Vec::new();
                let lines = element.text().collect::<Vec<_>>();
                
                for i in 2..lines.len() {
                    let line = lines[i].to_owned();

                    if !(lines[i].eq("\n") && i + 1 < lines.len() && lines[i + 1].eq("\n"))
                    {
                        if !line.contains("freestar.config") {
                            lyrics.push(line.trim().to_owned());
                        }
                    }
                }

                return Some(lyrics);
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

fn format_az_metadata(dat: &str) -> String {
    dat.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}