use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(Debug, PartialEq, Clone, Copy, EnumIter, Display, Serialize, Deserialize)]
pub enum Color {
    Red,
    Orange,
    Yellow,
    Lime,
    Green,
    Blue,
    Cyan,
    Magenta,
    Purple,
    Black,
    Gray,
    White,
}
impl Default for Color {
    fn default() -> Self {
        Self::White
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DrawCanvas {
    pub width: usize,
    pub height: usize,
    pub grid: Vec<Color>,
}
impl Default for DrawCanvas {
    fn default() -> Self {
        Self {
            width: 12,
            height: 12,
            grid: vec![Color::default(); 12 * 12],
        }
    }
}
impl DrawCanvas {
    pub fn set_pixel(&mut self, i: usize, color: Color) {
        self.grid[i] = color;
    }
    pub fn clear(&mut self) {
        self.grid = vec![Color::default(); self.width * self.height];
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GameState {
    pub prompt: String,
    pub canvas: DrawCanvas,
    pub players: Vec<()>,
}
impl GameState {
    pub fn new() -> Self {
        let mut rng: StdRng = SeedableRng::from_entropy();
        Self {
            prompt: FRUITS[rng.gen_range(0..FRUITS.len())].into(),
            canvas: DrawCanvas::default(),
            players: vec![],
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SetPixelPost {
    pub pixel_id: usize,
    pub color: Color,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub username: String,
    pub text: String,
}

pub const FRUITS: &[&str] = &[
    "Apple",
    "Apricot",
    "Artichoke",
    "Avocado",
    "Banana",
    "Beet",
    "Bell pepper",
    "Blackberry",
    "Blueberry",
    "Broccoli",
    "Brussels sprouts",
    "Cabbage",
    "Carrot",
    "Cauliflower",
    "Cherry",
    "Corn",
    "Cucumber",
    "Eggplant",
    "Fennel",
    "Garlic",
    "Grapefruit",
    "Grapes",
    "Honeydew melon",
    "Kale",
    "Kiwi",
    "Leek",
    "Lemon",
    "Lettuce",
    "Mango",
    "Mandarin",
    "Nectarine",
    "Onion",
    "Orange",
    "Papaya",
    "Parsnip",
    "Peach",
    "Pear",
    "Peas",
    "Pineapple",
    "Plum",
    "Pomegranate",
    "Potato",
    "Pumpkin",
    "Raisins",
    "Radish",
    "Raspberry",
    "Rhubarb",
    "Spinach",
    "Squash",
    "Strawberry",
    "Sweet potato",
    "Tomato",
    "Turnip",
    "Watermelon",
];
