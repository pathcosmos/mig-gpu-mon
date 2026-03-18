use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Quit,
}

pub struct EventHandler;

impl EventHandler {
    pub fn new() -> Self {
        Self
    }

    /// Poll for next event, blocking up to `timeout`.
    pub fn next(&self, timeout: Duration) -> Result<AppEvent> {
        if event::poll(timeout)? {
            match event::read()? {
                CrosstermEvent::Key(key) => {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        return Ok(AppEvent::Quit);
                    }
                    if key.code == KeyCode::Char('q') {
                        return Ok(AppEvent::Quit);
                    }
                    Ok(AppEvent::Key(key))
                }
                _ => Ok(AppEvent::Tick),
            }
        } else {
            Ok(AppEvent::Tick)
        }
    }
}
