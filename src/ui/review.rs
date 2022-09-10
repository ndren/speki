use rusqlite::Connection;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction::{Vertical, Horizontal}, Layout, Rect},
    style::{Color, Style, Modifier},
    widgets::{Block, Borders, ListItem, List},
    text::Spans,
    Frame,
};
use crate::utils::widgets::{
 //   card_status::card_status,
    view_dependents::view_dependents,
    view_dependencies::view_dependencies,
    button::draw_button,
    message_box::draw_message,
    progress_bar::progress_bar,
    cardlist::CardItem,
    mode_status::mode_status,

};

use crate::{
    app::App,
    logic::review::{
        ReviewMode,
        ReviewSelection,
        CardReview,
        UnfCard,
        IncMode,
        UnfSelection,
        IncSelection,
    },
    utils::{
        statelist::StatefulList,
        incread::IncListItem,
        misc::modecolor,
    }
};


//use crate::utils::widgets::find_card::draw_find_card;

pub fn main_review<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{

    let chunks = Layout::default()
        .direction(Vertical)
        .constraints(
            [
            Constraint::Ratio(1, 10),
            Constraint::Ratio(7, 10),
            ]
            .as_ref(),
            )
        .split(area);

    let (progbar, area) = (chunks[0], chunks[1]);

    let chunks = Layout::default()
        .direction(Vertical)
        .constraints(
            [
            Constraint::Max(1),
            Constraint::Ratio(7, 10),
            ]
            .as_ref(),
            )
        .split(progbar);

    let (status, progbar) = (chunks[0], chunks[1]);



    mode_status(f, status, &app.review.mode, &app.review.for_review, &app.review.start_qty);
    draw_progress_bar(f, app, progbar);


    match &mut app.review.mode{
        ReviewMode::Done                   => draw_done(f, app, area),
        ReviewMode::Review(review)         => draw_review(f, &app.conn, &review, area),
        ReviewMode::Pending(pending)       => draw_review(f, &app.conn, &pending, area),
        ReviewMode::Unfinished(unfinished) => draw_unfinished(f, &app.conn, &unfinished, area),
        ReviewMode::IncRead(inc)           => draw_incread(f, &app.conn,  inc, area),
    }

}

pub fn draw_unfinished<B>(f: &mut Frame<B>, conn: &Connection, unfinished: &UnfCard, area: Rect)
where
    B: Backend,
{

    let area = unfinished_layout(area);
    let selected = UnfSelect::new(&unfinished.selection);
    view_dependencies(f, unfinished.id, conn, area.dependencies,selected.dependencies); 
    view_dependents(f,   unfinished.id, conn, area.dependents, selected.dependents);
    unfinished.question.draw_field(f, area.question,  selected.question);
    unfinished.answer.draw_field(f,   area.answer,    selected.answer);
    draw_button(f, area.skip,   "skip",   selected.skip);
    draw_button(f, area.finish, "finish", selected.finish);
}


pub fn draw_incread<B>(f: &mut Frame<B>, _conn: &Connection, inc: &mut IncMode, area: Rect)
where
    B: Backend,
{

    let area = inc_layout(area);
    let selected = IncSelect::new(&inc.selection);

//    _app.review.incread.unwrap().source.draw_field(f, editing, "hey", Alignment::Left, false);


    inc.source.source.rowlen = area.source.width - 2;
    inc.source.source.window_height = area.source.height - 2;


    inc.source.source.draw_field(f, area.source, selected.source);
    let clozes: StatefulList<CardItem> = inc.source.clozes.clone();
    let list = {
        let bordercolor = if selected.clozes {Color::Red} else {Color::White};
        let style = Style::default().fg(bordercolor);

        let items: Vec<ListItem> = clozes.items.iter().map(|card| {
            let lines = vec![Spans::from(card.question.clone())];
            ListItem::new(lines)
                .style(Style::default())})
            .collect();
        
        let items = List::new(items)
            .block(Block::default()
                   .borders(Borders::ALL)
                   .border_style(style)
                   .title("Clozes"));
        
        if selected.clozes{
        items
            .highlight_style(
                Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )}
        else {items}
    };
    let mut state = clozes.state;
    f.render_stateful_widget(list, area.clozes, &mut state);


    let clozes: StatefulList<IncListItem> = inc.source.extracts.clone();
    let list = {
        let bordercolor = if selected.extracts {Color::Red} else {Color::White};
        let style = Style::default().fg(bordercolor);

        let items: Vec<ListItem> = clozes.items.iter().map(|card| {
            let lines = vec![Spans::from(card.text.clone())];
            ListItem::new(lines)
                .style(Style::default())})
            .collect();
        
        let items = List::new(items)
            .block(Block::default()
                   .borders(Borders::ALL)
                   .border_style(style)
                   .title("Extracts"));
        
        if selected.extracts{
        items
            .highlight_style(
                Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )}
        else {items}
    };
    let mut state = clozes.state;
    f.render_stateful_widget(list, area.extracts, &mut state);


draw_button(f, area.next,   "next", selected.skip);
draw_button(f, area.finish, "done", selected.complete);

}







pub fn draw_done<B>(f: &mut Frame<B>, _app: &mut App, area: Rect)
where
    B: Backend,
{
    draw_message(f, area, "Nothing left to review now!");
}




pub fn draw_progress_bar<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    

    let target = match app.review.mode{
        ReviewMode::Done       => 1,
        ReviewMode::Review(_)  => app.review.start_qty.fin_qty,
        ReviewMode::Pending(_) => app.review.start_qty.pending_qty,
        ReviewMode::Unfinished(_) => app.review.start_qty.unf_qty,
        ReviewMode::IncRead(_) => app.review.start_qty.inc_qty,
    } as u32;

    let current = match app.review.mode{
        ReviewMode::Done       => 0,
        ReviewMode::Review(_)  => app.review.for_review.review_cards.len(),
        ReviewMode::Pending(_) => app.review.for_review.pending_cards.len(),
        ReviewMode::Unfinished(_) => app.review.for_review.unfinished_cards.len(),
        ReviewMode::IncRead(_) => app.review.for_review.active_increads.len(),
    } as u32;

    let color = modecolor(&app.review.mode);



        progress_bar(f, current, target, color, area);
}

pub fn draw_review<B>(f: &mut Frame<B>, conn: &Connection, review: &CardReview, area: Rect)
where
    B: Backend,
{
    
    let area = review_layout(area);
    let selected = RevSelect::new(&review.selection);



   // card_status(f, app, area.status, false);
    review.question.draw_field(f, area.question, selected.question);
    if review.reveal{
        review.answer.draw_field(f, area.answer, selected.answer);
    } else {
        draw_message(f, area.answer, "Space to reveal");
    }
    view_dependencies(f, review.id, conn, area.dependencies, selected.dependencies); 
    view_dependents(f,   review.id, conn, area.dependents, selected.dependents);

}



struct IncSelect{
    source: bool,
    extracts: bool,
    clozes: bool,
    skip: bool,
    complete: bool,
}

impl IncSelect{
    fn new(choice: &IncSelection) -> Self{
        use IncSelection::*;

        let mut sel = IncSelect{
            source: false,
            extracts: false,
            clozes: false,
            skip: false,
            complete: false,
        };

        match choice{
            Source(_) => sel.source = true,
            Extracts(_) => sel.extracts = true,
            Clozes(_) => sel.clozes = true,
            Skip => sel.skip = true,
            Complete => sel.complete = true,
        }
        sel
    }
}
struct RevSelect{
    question: bool,
    answer: bool,
    dependents: bool,
    dependencies: bool,
}

impl RevSelect{
    fn new(choice: &ReviewSelection) -> Self{
        use ReviewSelection::*;

        let mut sel = RevSelect{
            question: false, 
            answer: false, 
            dependents: false,
            dependencies: false, 
        };

        match choice{
            Question(_)     => sel.question = true,
            Answer(_)       => sel.answer = true,
            Dependencies(_) => sel.dependencies = true,
            Dependents(_)   => sel.dependents = true,
        }
        sel
    }
}


struct UnfSelect{
    question: bool,
    answer: bool,
    dependents: bool,
    dependencies: bool,
    skip: bool,
    finish: bool,
}

impl UnfSelect{
    fn new(choice: &UnfSelection) -> Self{
        use UnfSelection::*;

        let mut sel = UnfSelect{
            question: false, 
            answer: false, 
            dependents: false,
            dependencies: false, 
            skip: false,
            finish: false,
        };

        match choice{
            Question(_)     => sel.question = true,
            Answer(_)       => sel.answer = true,
            Dependencies(_) => sel.dependencies = true,
            Dependents(_)   => sel.dependents = true,
            Skip            => sel.skip = true,
            Complete        => sel.finish = true,
        }
        sel
    }
}





struct DrawUnf{
    question: Rect,
    answer: Rect,
    dependencies: Rect,
    dependents: Rect,
    skip: Rect,
    finish: Rect,
}
struct DrawReview{
    question: Rect,
    answer: Rect,
    dependents: Rect,
    dependencies: Rect,
    _status: Rect,
}

struct DrawInc{
    source: Rect,
    extracts: Rect,
    clozes: Rect,
    next: Rect,
    finish: Rect,
}


fn inc_layout(area: Rect) -> DrawInc {
    let foobar = Layout::default()
        .direction(Vertical)
        .constraints(
            [
            Constraint::Ratio(8, 10),
            Constraint::Ratio(1, 10),
            ]
            .as_ref(),
            )
        .split(area);

    
    let (main, buttons) = (foobar[0], foobar[1]);
    
    let mainvec = Layout::default()
        .direction(Horizontal)
        .constraints(
            [
            Constraint::Ratio(3, 4),
            Constraint::Ratio(1, 4),
            ]
            .as_ref(),
            )
        .split(main);

    let buttons = Layout::default()
        .direction(Horizontal)
        .constraints(
            [
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2),
            ]
            .as_ref(),
            )
        .split(buttons);

    let (next_button, done_button) = (buttons[0], buttons[1]);
    
    let (editing, rightside) = (mainvec[0], mainvec[1]);

    let rightvec = Layout::default()
        .direction(Vertical)
        .constraints(
            [
            Constraint::Ratio(1, 9),
            Constraint::Ratio(4, 9),
            Constraint::Ratio(4, 9),
            ]
            .as_ref(),
            )
        .split(rightside);

    DrawInc { 
        source: editing,
        extracts: rightvec[1],
        clozes: rightvec[2],
        next: next_button,
        finish: done_button,
    }
}


fn unfinished_layout(area: Rect) -> DrawUnf {
    let foobar = Layout::default()
        .direction(Vertical)
        .constraints(
            [
            Constraint::Ratio(9, 12),
            Constraint::Ratio(1, 12),
            ]
            .as_ref(),
            )
        .split(area);
    
    let leftright = Layout::default()
        .direction(Horizontal)
        .constraints(
            [
            Constraint::Ratio(2, 3),
            Constraint::Ratio(1, 3),
            ]
                     .as_ref(),)
        .split(foobar[0]);

    let left = leftright[0];
    let right = leftright[1];

    let rightcolumn = Layout::default()
        .direction(Vertical)
        .constraints([Constraint::Ratio(1, 2),Constraint::Ratio(1, 2)]
                     .as_ref(),)
        .split(right);

    let leftcolumn = Layout::default()
        .constraints([Constraint::Ratio(1, 2),Constraint::Ratio(1, 2)]
                     .as_ref(),)
        .split(left);

    let bottom = Layout::default()
        .direction(Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3),]
                     .as_ref(),)
        .split(foobar[1]);



    DrawUnf{
        question: leftcolumn[0],
        answer:   leftcolumn[1],
        dependents: rightcolumn[0],
        dependencies: rightcolumn[1],
        skip: bottom[0],
        finish: bottom[1],


    }

}


fn review_layout(area: Rect) -> DrawReview{
    let updown = Layout::default()
        .direction(Vertical)
        .constraints(
            [
            Constraint::Ratio(9, 12),
            Constraint::Ratio(1, 12)
            ]
            .as_ref(),
            )
        .split(area);
    

    let leftright = Layout::default()
        .direction(Horizontal)
        .constraints(
            [
            Constraint::Ratio(2, 3),
            Constraint::Ratio(1, 3),
            ]
            .as_ref(),
            )
        .split(updown[0]);

    let left = leftright[0];
    let right = leftright[1];

    let rightcolumn = Layout::default()
        .direction(Vertical)
        .constraints(
            [
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2)
            ]
            .as_ref(),
            )
        .split(right);

    let leftcolumn = Layout::default()
        .constraints(
            [
            Constraint::Ratio(1, 2),
            Constraint::Ratio(1, 2)
            ]
            .as_ref(),
            )
        .split(left);

    DrawReview {
        question: leftcolumn[0],
        answer:   leftcolumn[1],
        dependents: rightcolumn[0],
        dependencies: rightcolumn[1],
        _status: updown[1],
    }

}
