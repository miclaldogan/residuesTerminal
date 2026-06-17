use ratatui::{
    layout::Rect,
    style::{Color, Style},
    Frame,
};
use super::state::GlobalStateContext;
use crossterm::event::KeyEvent;

pub fn render_workspace(f: &mut Frame, area: Rect, _state: &mut GlobalStateContext) {
    let buf = f.buffer_mut();
    let bg = Color::Rgb(15, 15, 15);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if x < buf.area().width && y < buf.area().height {
                let cell = buf.get_mut(x, y);
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg));
            }
        }
    }
    
    let msg = " [ ACT VI: TURING - PLACEHOLDER ] ";
    let style = Style::default().fg(Color::Rgb(220, 220, 220)).bg(bg);
    let x = area.x + (area.width.saturating_sub(msg.len() as u16) / 2);
    let y = area.y + area.height / 2;
    
    let mut col = x;
    for ch in msg.chars() {
        if col >= area.x + area.width { break; }
        if col < buf.area().width && y < buf.area().height {
            let cell = buf.get_mut(col, y);
            cell.set_char(ch);
            cell.set_style(style);
        }
        col += 1;
    }
}

pub fn handle_input(_key: KeyEvent, _state: &mut GlobalStateContext) {}
