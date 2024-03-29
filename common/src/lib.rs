use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[derive(Debug, Display, Clone, Copy, PartialEq, EnumIter, Serialize, Deserialize)]
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
    pub players: Vec<Player>,
}
impl GameState {
    pub fn new() -> Self {
        Self {
            prompt: Self::random_prompt(None),
            canvas: DrawCanvas::default(),
            players: vec![],
        }
    }
    fn random_prompt(not: Option<&str>) -> String {
        let mut rng: StdRng = SeedableRng::from_entropy();
        loop {
            let p = FRUITS[rng.gen_range(0..FRUITS.len())].to_lowercase();
            match not {
                Some(prev) => {
                    if p != prev {
                        return p;
                    }
                }
                None => return p,
            }
        }
    }
    /// Returns whether player was added
    pub fn add_player(&mut self, mut player: Player) -> bool {
        if self.players.contains(&player) {
            return false;
        }
        player.active = self.players.is_empty();
        self.players.push(player);
        true
    }
    /// Returns whether game should move to next round
    pub fn remove_player(&mut self, player: Player) -> bool {
        self.players
            .iter()
            .find(|p| **p == player)
            .map(|p| p.active)
            .unwrap_or_default()
    }
    pub fn new_round(&mut self) {
        self.canvas.clear();
        self.prompt = Self::random_prompt(Some(&self.prompt));
        let i = match self.players.iter_mut().position(|p| {
            let b = p.active;
            p.active = false;
            b
        }) {
            Some(j) => (j + 1) % self.players.len(),
            None => 0,
        };
        self.players[i].active = true;
    }
}
impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameInfo {
    pub room_id: String,
    pub prompt: String,
    pub players: Vec<Player>,
}

#[derive(Serialize, Deserialize)]
pub struct SessionPlayer {
    pub username: String,
    pub room: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub username: String,
    pub active: bool,
}
impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.username == other.username
    }
}

#[derive(Serialize, Deserialize)]
pub struct SetPixelPost {
    pub pixel_id: usize,
    pub color: Color,
}

#[derive(Serialize, Deserialize)]
pub struct JoinLobbyPost {
    pub username: String,
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
