use iced::{
   button, container, executor, scrollable, text_input, Align, Application, Button, Column, Command, Container, Element, Length, Scrollable, Settings, Text, TextInput, Color, Background,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;


const PYTHON_NLP_SCRIPT: &str = r#"

from llama_cpp import Llama
from spacy import load

nlp = load("en_core_web_sm")

def determine_token(chunk):
   doc = nlp(chunk)
   entities = [(ent.text, ent.label_) for ent in doc.ents]
   verbs = [token.text for token in doc if token.pos_ == "VERB"]
   return "[action]" if verbs else "[attention]", entities

def llama_generate(prompt, max_tokens, chunk_size):
   doc = nlp(prompt)
   sentences = list(doc.sents)
   responses = []
   for sentence in sentences:
       chunk = sentence.text
       token, entities = determine_token(chunk)
       responses.append(f"Chunk: {chunk}, Token: {token}, Entities: {entities}")
   return ' '.join(responses)
"#;

#[pyfunction]
fn llama_generate_rust(prompt: String, max_tokens: usize, chunk_size: usize) -> PyResult<String> {
   Python::with_gil(|py| {
       let sqlite3 = py.import("sqlite3")?;
       let conn = sqlite3.call_method0("connect", ("chat_history.db",))?;
       let cursor = conn.call_method0("cursor", ())?;
       cursor.call_method1("execute", ("SELECT message FROM chat_history ORDER BY ROWID DESC LIMIT 3",))?;
       let past_context = cursor.call_method0("fetchall", ())?;
       let past_context = past_context.into_iter().map(|row| row.get_item(0).extract::<String>().unwrap()).collect::<Vec<String>>().join(" ");

       let py_globals = PyDict::new(py);
       py.run(PYTHON_NLP_SCRIPT, Some(py_globals), None)?;
       let generate_func = py_globals.get_item("llama_generate").unwrap().to_object(py);
       let result = generate_func.call1(py, (prompt, max_tokens, chunk_size, past_context))?;
       result.extract::<String>()
   }).map_err(|e| {
       println!("Error occurred while running Python script: {:?}", e);
       e
   })
}

fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
   match message {
       Message::SendPressed => {
           let response = format!("AI: {}", self.message_input_value);
           self.response_history.push(response);
           save_chat_history(&self.response_history).map_err(|e| {
               println!("Error occurred while saving chat history: {:?}", e);
               e
           })?;
           self.message_input_value.clear();
       }
       Message::InputChanged(value) => {
           self.message_input_value = value;
       }
   }
   Command::none()
}

fn save_chat_history(history: &Vec<String>) -> PyResult<()> {
   Python::with_gil(|py| {
       let sqlite3 = py.import("sqlite3")?;
       let conn = sqlite3.call_method0("connect", ("chat_history.db",))?;
       let cursor = conn.call_method0("cursor", ())?;
       cursor.call_method1("execute", ("CREATE TABLE IF NOT EXISTS chat_history (message TEXT)",))?;
       for message in history {
           cursor.call_method1("execute", (format!("INSERT INTO chat_history (message) VALUES ('{}')", message),))?;
       }
       conn.call_method0("commit", ())?;
       Ok(())
   })
}

struct ChatApp {
    send_button: button::State,
    message_input: text_input::State,
    message_input_value: String,
    response_history: Vec<String>,
    scroll: scrollable::State,
}

#[derive(Debug, Clone)]
enum Message {
    SendPressed,
    InputChanged(String),
}

impl Application for ChatApp {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (
            Self {
                send_button: button::State::new(),
                message_input: text_input::State::new(),
                message_input_value: String::new(),
                response_history: Vec::new(),
                scroll: scrollable::State::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("ChatApp - Rust with AI")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::SendPressed => {
                let response = format!("AI: {}", self.message_input_value);
                self.response_history.push(response);
                self.message_input_value.clear();
            }
            Message::InputChanged(value) => {
                self.message_input_value = value;
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        let input = TextInput::new(
            &mut self.message_input,
            "Type a message...",
            &self.message_input_value,
            Message::InputChanged,
        )
        .padding(10)
        .size(20)
        .style(DarkTextInput);

        let send_button = Button::new(&mut self.send_button, Text::new("Send"))
            .on_press(Message::SendPressed)
            .style(DarkButton);

        let mut content = Column::new()
            .align_items(Align::Center)
            .spacing(20)
            .push(input)
            .push(send_button);

        for response in &self.response_history {
            content = content.push(Text::new(response.clone()).color(Color::WHITE));
        }

        let scrollable_content = Scrollable::new(&mut self.scroll)
            .padding(20)
            .push(content)
            .style(DarkScrollable);

        Container::new(scrollable_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(DarkContainer)
            .into()
    }
}
// Dark-themed button styles
struct DarkButton;
impl button::StyleSheet for DarkButton {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
            border_radius: 5.0,
            text_color: Color::WHITE,
            ..button::Style::default()
        }
    }


}


struct DarkTextInput;
impl text_input::StyleSheet for DarkTextInput {
    fn active(&self) -> text_input::Style {
        text_input::Style {
            background: Background::Color(Color::from_rgb(0.2, 0.2, 0.2)),
            border_radius: 5.0,
            border_width: 1.0,
            border_color: Color::from_rgb(0.7, 0.7, 0.7),
            ..text_input::Style::default()
        }
    }


}


struct DarkScrollable;
impl scrollable::StyleSheet for DarkScrollable {
    fn active(&self) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: Some(Background::Color(Color::from_rgb(0.1, 0.1, 0.1))),
            border_radius: 5.0,
            border_width: 0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: Color::from_rgb(0.5, 0.5, 0.5),
                border_radius: 5.0,
                border_width: 0,
                border_color: Color::TRANSPARENT,
            },
            ..scrollable::Scrollbar::default()
        }
    }


}


struct DarkContainer;
impl container::StyleSheet for DarkContainer {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(Color::from_rgb(0.1, 0.1, 0.1))),
            text_color: Some(Color::WHITE),
            ..container::Style::default()
        }
    }
}

fn main() -> iced::Result {
   ChatApp::run(Settings::default()).map_err(|e| {
       println!("Error occurred while running Rust application: {:?}", e);
       e
   })
}
