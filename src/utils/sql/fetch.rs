use crate::utils::aliases::*;
use crate::utils::card::{Card, RecallGrade, Review}; //, Topic, Review}
use crate::widgets::load_cards::MediaContents;
use crate::widgets::topics::Topic;
use rusqlite::{Connection, Result, Row};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

enum CardFilter {
    Suspended(bool),
    Resolved(bool),
    Finished(bool),
    Unfinished(bool),
    Pending(bool),
    Cardtype(CardType),
    StrengthRange((f32, f32)),
    Minstability(u32),
    Maxstability(u32),
    Contains(String),
    Topics(Vec<TopicID>),
    MinPosition(u32),
    MaxPosition(u32),
    MinSkipDaysPassed(f32),
    MaxSkipDaysPassed(f32),
    Source(u32),
    DueUnfinished,
}

use std::fmt;
impl fmt::Display for CardFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use CardFilter::*;
        let text = match self {
            Suspended(val) => format!("suspended = {}", val),
            Resolved(val) => format!("resolved = {}", val),
            Cardtype(val) => match val {
                CardType::Pending => "cardtype = 0".to_string(),
                CardType::Unfinished => "cardtype = 1".to_string(),
                CardType::Finished => "cardtype = 2".to_string(),
            },
            Pending(val) => match val {
                true => "cardtype = 0".to_string(),
                false => "cardtype != 0".to_string(),
            },
            Unfinished(val) => match val {
                true => "cardtype = 1".to_string(),
                false => "cardtype != 1".to_string(),
            },
            Finished(val) => match val {
                true => "cardtype = 2".to_string(),
                false => "cardtype != 2".to_string(),
            },
            DueUnfinished => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as f32;
                format!("{} - skiptime > (skipduration * 84600)", now)
            }
            MaxSkipDaysPassed(val) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as f32;
                let since = now - (val * 86400.);
                format!("skiptime < {}", since)
            }
            MinSkipDaysPassed(val) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as f32;
                let since = now - (val * 86400.);
                format!("skiptime > {}", since)
            }
            Source(val) => format!("source = {}", val),
            MaxPosition(val) => format!("position < {}", val),
            MinPosition(val) => format!("position > {}", val),
            Minstability(val) => format!("stability > {}", val),
            Maxstability(val) => format!("stability < {}", val),
            StrengthRange(val) => format!("strength BETWEEN {} and {}", val.0, val.1),
            Contains(val) => format!("(question LIKE '%{}%' or answer LIKE '%{}%')", val, val),
            Topics(vec) => {
                let mut topicstr = String::from("(");
                for id in vec {
                    topicstr.push_str(&format!("topic = {} OR ", id));
                }
                // removes the final " OR "
                for _ in 0..4 {
                    topicstr.pop();
                }
                topicstr.push(')');
                topicstr
            }
        };
        write!(f, "{}", text)
    }
}

#[derive(Default)]
pub struct CardQuery {
    filters: Vec<CardFilter>,
    limit: Option<u32>,
}

impl CardQuery {
    fn make_query(&self) -> String {
        let mut query = r#"SELECT * 
            FROM cards
            LEFT OUTER JOIN finished_cards
            ON cards.id = finished_cards.id 
            LEFT OUTER JOIN unfinished_cards
            ON cards.id = unfinished_cards.id 
            LEFT OUTER JOIN pending_cards
            ON cards.id = pending_cards.id

            "#
        .to_string();
        if !self.filters.is_empty() {
            query.push_str("WHERE ");
            for filter in &self.filters {
                query.push_str(&format!("{} and ", filter));
            }

            // removes the final " and "
            for _ in 0..4 {
                query.pop();
            }
        }
        if let Some(limit) = self.limit {
            query.push_str(&format!("LIMIT {}", limit));
        }
        query
    }

    pub fn suspended(mut self, val: bool) -> Self {
        self.filters.push(CardFilter::Suspended(val));
        self
    }
    pub fn resolved(mut self, val: bool) -> Self {
        self.filters.push(CardFilter::Resolved(val));
        self
    }
    pub fn finished(mut self, val: bool) -> Self {
        self.filters.push(CardFilter::Finished(val));
        self
    }
    pub fn unfinished(mut self, val: bool) -> Self {
        self.filters.push(CardFilter::Unfinished(val));
        self
    }
    pub fn pending(mut self, val: bool) -> Self {
        self.filters.push(CardFilter::Pending(val));
        self
    }
    pub fn strength(mut self, val: (f32, f32)) -> Self {
        self.filters.push(CardFilter::StrengthRange(val));
        self
    }
    pub fn minimum_stability(mut self, val: u32) -> Self {
        self.filters.push(CardFilter::Minstability(val));
        self
    }
    pub fn max_stability(mut self, val: u32) -> Self {
        self.filters.push(CardFilter::Maxstability(val));
        self
    }
    pub fn contains(mut self, val: String) -> Self {
        self.filters.push(CardFilter::Contains(val));
        self
    }
    pub fn topics(mut self, val: Vec<TopicID>) -> Self {
        self.filters.push(CardFilter::Topics(val));
        self
    }
    pub fn minimum_position(mut self, val: u32) -> Self {
        self.filters.push(CardFilter::MinPosition(val));
        self
    }
    pub fn max_position(mut self, val: u32) -> Self {
        self.filters.push(CardFilter::MaxPosition(val));
        self
    }
    pub fn minimum_days_since_skip(mut self, val: f32) -> Self {
        self.filters.push(CardFilter::MinSkipDaysPassed(val));
        self
    }
    pub fn max_days_since_skip(mut self, val: f32) -> Self {
        self.filters.push(CardFilter::MaxSkipDaysPassed(val));
        self
    }
    pub fn source(mut self, val: u32) -> Self {
        self.filters.push(CardFilter::Source(val));
        self
    }
    pub fn unfinished_due(mut self) -> Self {
        self.filters.push(CardFilter::DueUnfinished);
        self
    }
    pub fn cardtype(mut self, val: CardType) -> Self {
        self.filters.push(CardFilter::Cardtype(val));
        self
    }
    pub fn limit(mut self, val: u32) -> Self {
        self.limit = Some(val);
        self
    }
    pub fn fetch_card_ids(self, conn: &Arc<Mutex<Connection>>) -> Vec<CardID> {
        let query = self.make_query();
        let mut cardvec = Vec::<CardID>::new();
        conn.lock()
            .unwrap()
            .prepare(&query)
            .unwrap()
            .query_map([], |row| {
                cardvec.push(row.get(0).unwrap());
                Ok(())
            })
            .unwrap()
            .for_each(|_| {});
        cardvec
    }
    pub fn fetch_carditems(self, conn: &Arc<Mutex<Connection>>) -> Vec<CardItem> {
        let query = self.make_query();
        let mut cardvec = Vec::<CardItem>::new();
        conn.lock()
            .unwrap()
            .prepare(&query)
            .unwrap()
            .query_map([], |row| {
                cardvec.push(CardItem {
                    question: row.get(1).unwrap(),
                    id: row.get(0).unwrap(),
                });
                Ok(())
            })
            .unwrap()
            .for_each(|_| {});
        cardvec
    }
}

#[derive(Clone)]
pub struct DepPair {
    dependent: u32,
    dependency: u32,
}

pub fn prev_id(conn: &Arc<Mutex<Connection>>) -> Result<u32> {
    Ok(conn.lock().unwrap().last_insert_rowid() as u32)
}

pub fn fetch_question(conn: &Arc<Mutex<Connection>>, cid: CardID) -> String {
    fetch_card(conn, cid).question
}

pub fn get_topic_of_card(conn: &Arc<Mutex<Connection>>, cid: CardID) -> TopicID {
    fetch_card(conn, cid).topic
}

pub fn fill_dependencies(conn: &Arc<Mutex<Connection>>, mut card: Card) -> Card {
    card.dependents = get_dependents(conn, card.id).unwrap();
    card.dependencies = get_dependencies(conn, card.id).unwrap();
    card
}

pub fn fetch_card(conn: &Arc<Mutex<Connection>>, cid: u32) -> Card {
    let card = conn
        .lock()
        .unwrap()
        .query_row("SELECT * FROM cards WHERE id=?", [cid], |row| row2card(row))
        .expect(&format!("Failed to query following card: {}", cid));
    fill_dependencies(conn, card)
}

pub fn get_topics(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Topic>> {
    let mut vecoftops = Vec::<Topic>::new();
    conn.lock()
        .unwrap()
        .prepare("SELECT * FROM topics")?
        .query_map([], |row| {
            Ok(Topic {
                id: row.get(0)?,
                name: row.get(1)?,
                parent: row.get(2)?,
                children: Vec::<CardID>::new(),
                ancestors: 0,
                relpos: row.get(3)?,
            })
        })?
        .for_each(|topic| {
            vecoftops.push(topic.unwrap());
        });
    Ok(vecoftops)
}

pub fn get_history(conn: &Arc<Mutex<Connection>>, id: u32) -> Result<Vec<Review>> {
    let mut vecofrows = Vec::<Review>::new();
    conn.lock()
        .unwrap()
        .prepare("SELECT * FROM revlog WHERE cid = ?")?
        .query_map([id], |row| {
            vecofrows.push(Review {
                grade: RecallGrade::from(row.get(2)?).unwrap(),
                date: row.get(0)?,
                answertime: row.get(3)?,
            });
            Ok(())
        })?
        .for_each(|_| {});
    Ok(vecofrows)
}

// pub fn row2card(conn: &Arc<Mutex<Connection>>, row: &Row) -> Result<Card>{

pub fn row2card(row: &Row) -> Result<Card> {
    let cardtype = match row.get::<usize, u32>(7)? {
        0 => CardType::Pending,
        1 => CardType::Unfinished,
        2 => CardType::Finished,
        _ => panic!(),
    };
    let id = row.get(0)?;

    let frontaudio: Option<String> = row.get(3)?;
    let backaudio: Option<String> = row.get(4)?;
    let frontimage: Option<String> = row.get(5)?;
    let backimage: Option<String> = row.get(6)?;

    let frontaudio: Option<PathBuf> = frontaudio.map(|x| PathBuf::from(x));
    let backaudio: Option<PathBuf> = backaudio.map(|x| PathBuf::from(x));
    let frontimage: Option<PathBuf> = frontimage.map(|x| PathBuf::from(x));
    let backimage: Option<PathBuf> = backimage.map(|x| PathBuf::from(x));

    //  let dependencies = get_dependencies(conn, id).unwrap();
    //  let dependents = get_depndents(conn, id).unwrap();
    Ok(Card {
        id,
        question: row.get(1)?,
        answer: row.get(2)?,
        frontaudio,
        backaudio,
        frontimage,
        backimage,
        cardtype,
        suspended: row.get(8)?,
        resolved: row.get(9)?,
        dependents: Vec::new(),
        dependencies: Vec::new(),
        topic: row.get(10)?,
        source: row.get(11)?,
    })
}

pub fn load_cards(conn: &Arc<Mutex<Connection>>) -> Result<Vec<Card>> {
    let mut cardvec = Vec::<Card>::new();
    conn.lock()
        .unwrap()
        .prepare("SELECT * FROM cards")?
        .query_map([], |row| {
            cardvec.push(row2card(row).unwrap());
            Ok(())
        })?
        .for_each(|_| {});
    for i in 0..cardvec.len() {
        let id = cardvec[i].id;
        cardvec[i].dependencies = get_dependencies(conn, id).unwrap();
        cardvec[i].dependents = get_dependents(conn, id).unwrap();
    }

    Ok(cardvec)
}

pub fn load_card_matches(conn: &Arc<Mutex<Connection>>, search: &str) -> Result<Vec<Card>> {
    let mut cardvec = Vec::<Card>::new();
    conn
        .lock()
        .unwrap()
        .prepare("SELECT * FROM cards WHERE (question LIKE '%' || ?1 || '%') OR (answer LIKE '%' || ?1 || '%') LIMIT 50")?
        .query_map([search], |row| {
            cardvec.push(row2card(row).unwrap());
            Ok(())
        })?.for_each(|_|{});
    for i in 0..cardvec.len() {
        let id = cardvec[i].id;
        cardvec[i].dependencies = get_dependencies(conn, id).unwrap();
        cardvec[i].dependents = get_dependents(conn, id).unwrap();
    }
    Ok(cardvec)
}

pub fn get_dependents(conn: &Arc<Mutex<Connection>>, dependency: u32) -> Result<Vec<u32>> {
    let mut depvec = Vec::<CardID>::new();
    conn.lock()
        .unwrap()
        .prepare("SELECT dependent FROM dependencies where dependency = ?")?
        .query_map([dependency], |row| {
            depvec.push(row.get(0).unwrap());
            Ok(())
        })?
        .for_each(|_| {});
    Ok(depvec)
}

pub fn get_dependencies(conn: &Arc<Mutex<Connection>>, dependent: CardID) -> Result<Vec<CardID>> {
    let mut depvec = Vec::<CardID>::new();
    conn.lock()
        .unwrap()
        .prepare("SELECT dependency FROM dependencies where dependent = ?")?
        .query_map([dependent], |row| {
            depvec.push(row.get(0).unwrap());
            Ok(())
        })?
        .for_each(|_| {});
    Ok(depvec)
}

use crate::utils::card::CardType;

use crate::utils::incread::IncRead;
use crate::utils::statelist::StatefulList;
use crate::widgets::textinput::Field;

struct IncTemp {
    parent: u32,
    topic: u32,
    source: String,
    isactive: bool,
}

pub fn fetch_media(conn: &Arc<Mutex<Connection>>, id: CardID) -> MediaContents {
    let card = fetch_card(conn, id);
    MediaContents {
        frontaudio: card.frontaudio,
        backaudio: card.backaudio,
        frontimage: card.frontimage,
        backimage: card.backimage,
    }
}

// -------------------------------------------------------------- //

pub fn get_incread(conn: &Arc<Mutex<Connection>>, id: u32) -> Result<IncRead> {
    let extracts = load_extracts(conn, id).unwrap();
    let cloze_cards = CardQuery::default().source(id).fetch_carditems(conn);
    conn.lock()
        .unwrap()
        .query_row("SELECT * FROM incread WHERE id = ?", [id], |row| {
            Ok(IncRead {
                id,
                parent: row.get(1)?,
                topic: row.get(2)?,
                source: Field::new_with_text(
                    row.get(3).unwrap(),
                    row.get(7).unwrap(),
                    row.get(8).unwrap(),
                ),
                extracts: StatefulList::with_items(extracts),
                clozes: StatefulList::with_items(cloze_cards),
                isactive: row.get(4)?,
            })
        })
}

use crate::utils::incread::IncListItem;

pub fn load_inc_items(conn: &Arc<Mutex<Connection>>, topic: TopicID) -> Result<Vec<IncListItem>> {
    let mut incvec = Vec::<IncListItem>::new();
    conn.lock()
        .unwrap()
        .prepare("SELECT * FROM incread where parent = 0 and topic = ?")
        .unwrap()
        .query_map([topic], |row| {
            incvec.push(IncListItem {
                text: row.get(3)?,
                id: row.get(0)?,
            });
            Ok(())
        })?
        .for_each(|_| {});
    Ok(incvec)
}

pub fn load_extracts(conn: &Arc<Mutex<Connection>>, parent: IncID) -> Result<Vec<IncListItem>> {
    let mut incvec = Vec::<IncListItem>::new();
    conn.lock()
        .unwrap()
        .prepare("SELECT * FROM incread where parent = ?")
        .unwrap()
        .query_map([parent], |row| {
            incvec.push(IncListItem {
                text: row.get(3)?,
                id: row.get(0)?,
            });
            Ok(())
        })?
        .for_each(|_| {});
    Ok(incvec)
}

pub fn load_active_inc(conn: &Arc<Mutex<Connection>>) -> Result<Vec<IncID>> {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u32;
    let mut incvec = Vec::<IncID>::new();
    conn.lock()
        .unwrap()
        .prepare(
            "SELECT id FROM incread where active = 1 and ((? - skiptime) > (skipduration * 86400))",
        )
        .unwrap()
        .query_map([current_time], |row| {
            incvec.push(row.get(0).unwrap());
            Ok(())
        })?
        .for_each(|_| {});
    Ok(incvec)
}

use crate::widgets::cardlist::CardItem;
pub fn load_inc_text(conn: &Arc<Mutex<Connection>>, id: IncID) -> Result<String> {
    conn.lock()
        .unwrap()
        .prepare("SELECT source FROM incread where id = ?")
        .unwrap()
        .query_row([id], |row| Ok(row.get(0).unwrap()))
}

pub fn load_inc_title(
    conn: &Arc<Mutex<Connection>>,
    incid: IncID,
    titlelen: u16,
) -> Result<String> {
    let mut source = load_inc_text(conn, incid).unwrap();
    source.truncate(titlelen.into());
    if source.len() < 5 {
        source = "Empty Source".to_string();
    }
    Ok(source)
}

pub fn get_topic_of_inc(conn: &Arc<Mutex<Connection>>, id: IncID) -> Result<TopicID> {
    conn.lock()
        .unwrap()
        .prepare("SELECT topic FROM incread where id = ?")
        .unwrap()
        .query_row([id], |row| Ok(row.get(0).unwrap()))
}

pub fn get_skipduration(conn: &Arc<Mutex<Connection>>, id: CardID) -> Result<u32> {
    conn.lock().unwrap().query_row(
        "select skipduration FROM unfinished_cards WHERE id=?",
        [id],
        |row| row.get(0),
    )
}

pub fn get_inc_skipduration(conn: &Arc<Mutex<Connection>>, id: IncID) -> Result<u32> {
    conn.lock()
        .unwrap()
        .query_row("select skipduration FROM incread WHERE id=?", [id], |row| {
            row.get(0)
        })
}

pub fn get_skiptime(conn: &Arc<Mutex<Connection>>, id: CardID) -> Result<u32> {
    conn.lock().unwrap().query_row(
        "select skiptime FROM unfinished_cards WHERE id=?",
        [id],
        |row| row.get(0),
    )
}

pub fn get_stability(conn: &Arc<Mutex<Connection>>, id: CardID) -> f32 {
    conn.lock()
        .unwrap()
        .query_row(
            "select stability FROM finished_cards WHERE id=?",
            [id],
            |row| row.get(0),
        )
        .expect(&format!(
            "Couldn't find stability of card with following id: {}",
            id
        ))
}

pub fn get_strength(conn: &Arc<Mutex<Connection>>, id: CardID) -> Result<f32> {
    conn.lock().unwrap().query_row(
        "select strength FROM finished_cards WHERE id=?",
        [id],
        |row| row.get(0),
    )
}

pub fn is_resolved(conn: &Arc<Mutex<Connection>>, id: CardID) -> bool {
    conn.lock()
        .unwrap()
        .query_row("select resolved FROM cards WHERE id=?", [id], |row| {
            row.get(0)
        })
        .unwrap()
}

//use crate::utils::card::CardType;
pub fn get_cardtype(conn: &Arc<Mutex<Connection>>, id: CardID) -> CardType {
    match conn
        .lock()
        .unwrap()
        .query_row("select cardtype FROM cards WHERE id=?", [id], |row| {
            row.get::<usize, usize>(0)
        })
        .unwrap()
    {
        0 => CardType::Pending,
        1 => CardType::Unfinished,
        2 => CardType::Finished,
        _ => panic!(),
    }
}
