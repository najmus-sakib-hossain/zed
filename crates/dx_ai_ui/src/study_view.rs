//! Study View — Learning-focused AI assistant (Part 2).
//!
//! Provides:
//! - Concept explanation with difficulty adjustment
//! - Socratic questioning mode
//! - Flashcard generation
//! - Quiz creation from study material
//! - Progress tracking across topics

use gpui::{div, prelude::*, SharedString, Window};

/// Learning assistant view.
pub struct StudyView {
    topic: String,
    difficulty: DifficultyLevel,
    mode: StudyMode,
    flashcards: Vec<Flashcard>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifficultyLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyMode {
    Explain,
    Socratic,
    Flashcard,
    Quiz,
}

#[derive(Debug, Clone)]
pub struct Flashcard {
    pub front: String,
    pub back: String,
    pub confidence: f32, // 0.0 = unknown, 1.0 = mastered
}

impl StudyView {
    pub fn new() -> Self {
        Self {
            topic: String::new(),
            difficulty: DifficultyLevel::Intermediate,
            mode: StudyMode::Explain,
            flashcards: Vec::new(),
        }
    }

    pub fn set_topic(&mut self, topic: String, cx: &mut Context<Self>) {
        self.topic = topic;
        cx.notify();
    }

    pub fn set_difficulty(&mut self, level: DifficultyLevel, cx: &mut Context<Self>) {
        self.difficulty = level;
        cx.notify();
    }

    pub fn set_mode(&mut self, mode: StudyMode, cx: &mut Context<Self>) {
        self.mode = mode;
        cx.notify();
    }

    pub fn add_flashcard(&mut self, card: Flashcard, cx: &mut Context<Self>) {
        self.flashcards.push(card);
        cx.notify();
    }
}

impl Render for StudyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let label: SharedString = if self.topic.is_empty() {
            "Choose a topic to study...".into()
        } else {
            format!(
                "Studying: {} ({:?}) — {:?} mode — {} flashcards",
                self.topic,
                self.difficulty,
                self.mode,
                self.flashcards.len()
            )
            .into()
        };

        div().size_full().flex().flex_col().child(label)
    }
}
