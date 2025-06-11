use std::collections::VecDeque;

use anyhow::Result;
use groqu::{ models::{ ChatCompletionRequest, ChatMessage }, Groq };
use rand::{ rngs::ThreadRng, seq::SliceRandom };

const COLORS: [Color; 4] = [Color::Red, Color::Green, Color::Blue, Color::Yellow];
const UNO_PROMPT: &'static str = include_str!(
    concat!(env!("CARGO_MANIFEST_DIR"), "/prompts/uno.md")
);
const GAMEPLAY_PROMPT: &'static str = include_str!(
    concat!(env!("CARGO_MANIFEST_DIR"), "/prompts/play.md")
);

const MODEL: &'static str = "mistral-saba-24b";
const TOKEN: &'static str = "gsk_yourGroqTokenOrSomething123";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Color {
    Red,
    Green,
    Blue,
    Yellow,
}

impl From<&str> for Color {
    fn from(value: &str) -> Self {
        match value {
            "red" => Color::Red,
            "green" => Color::Green,
            "blue" => Color::Blue,
            "yellow" => Color::Yellow,
            _ => panic!("Unknown color code"),
        }
    }
}

#[derive(Debug, Clone)]
enum CardType {
    Number(u8),
    Skip,
    Reverse,
    DrawTwo,
    Wild,
    WildDrawFour,
}

impl ToString for CardType {
    fn to_string(&self) -> String {
        match self {
            Self::Number(n) => n.to_string(),
            Self::Skip => format!("Skip"),
            Self::Reverse => format!("Reverse"),
            Self::DrawTwo => format!("+2"),
            Self::Wild => format!("WILD"),
            Self::WildDrawFour => format!("WILD +2"),
        }
    }
}

#[derive(Clone)]
struct Card {
    typ: CardType,
    color: Option<Color>,
}

impl Card {
    fn number(&self) -> Option<u8> {
        match self.typ {
            CardType::Number(n) => Some(n),
            _ => None,
        }
    }
}

impl ToString for Card {
    fn to_string(&self) -> String {
        if let Some(color) = &self.color {
            format!("({:?} {})", color, self.typ.to_string())
        } else {
            format!("({})", self.typ.to_string())
        }
    }
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

fn build_deck() -> VecDeque<Card> {
    let mut deck = VecDeque::new();
    for color in &COLORS {
        deck.push_back(Card {
            typ: CardType::Number(0),
            color: Some(color.clone()),
        });

        // Two for each 1-9 number cards
        for _ in 0..2 {
            for n in 1..=9 {
                deck.push_back(Card {
                    typ: CardType::Number(n),
                    color: Some(color.clone()),
                });
            }

            deck.push_back(Card {
                typ: CardType::Skip,
                color: Some(color.clone()),
            });
            deck.push_back(Card {
                typ: CardType::Reverse,
                color: Some(color.clone()),
            });
            deck.push_back(Card {
                typ: CardType::DrawTwo,
                color: Some(color.clone()),
            });
        }
    }

    // No color
    for _ in 0..4 {
        deck.push_back(Card {
            typ: CardType::Wild,
            color: None,
        });
        deck.push_back(Card {
            typ: CardType::WildDrawFour,
            color: None,
        });
    }

    deck
}

fn deal_cards(deck: &mut VecDeque<Card>, players: usize) -> Vec<(usize, VecDeque<Card>)> {
    let mut hands: Vec<(usize, VecDeque<Card>)> = vec![];
    for id in 0..players {
        let mut cards = VecDeque::new();

        for _ in 0..7 {
            if let Some(card) = deck.pop_front() {
                cards.push_back(card);
            }
        }

        hands.push((id, cards));
    }

    hands
}

#[derive(Debug, Clone)]
enum Effect {
    Nothing,
    Skip,
    Reverse,
    DrawTwo,
    Wild,
    WildDrawFour,
    Wrong,
}

impl Effect {
    fn as_str(&self) -> &str {
        match self {
            Self::Nothing => "No effect",
            Self::Skip => "Skip",
            Self::Reverse => "Reverse turn",
            Self::DrawTwo => "Draw 2 cards",
            Self::Wild => "WILD!",
            Self::WildDrawFour => "WILD, and draw 4 cards",
            Self::Wrong => unimplemented!("No &str available for incorrect usage"),
        }
    }
}

struct UnoGame {
    deck: VecDeque<Card>,
    table: VecDeque<Card>,
    rng: ThreadRng,
    turn: usize,
    hands: Vec<(usize, VecDeque<Card>)>,
    wins: usize,
}

impl UnoGame {
    fn new(n_players: usize) -> Self {
        assert!(2 <= n_players && n_players <= 10);

        let mut deck = build_deck();
        let mut rng = rand::rng();
        deck.make_contiguous().shuffle(&mut rng);

        let hands = deal_cards(&mut deck, n_players);
        Self { deck, table: VecDeque::new(), rng, turn: const { 0 }, hands, wins: const { 0 } }
    }

    fn view(&mut self, player: usize) -> &VecDeque<Card> {
        &self.hands[player].1
    }

    fn get_effect_after_put(&mut self, card: Card) -> (Effect, Card) {
        if let Some(last) = self.table.back() {
            if let Some(o) = card.number() {
                if let Some(lo) = last.number() {
                    if o == lo {
                        return (Effect::Nothing, card);
                    }
                }
            }

            if let Some(c) = &card.color {
                if let Some(co) = &last.color {
                    if c == co {
                        return (
                            match last.typ {
                                CardType::Number(_) => Effect::Nothing,
                                CardType::Skip => Effect::Skip,
                                CardType::Reverse => Effect::Reverse,
                                CardType::DrawTwo => Effect::DrawTwo,
                                CardType::Wild => Effect::Wild,
                                CardType::WildDrawFour => Effect::WildDrawFour,
                            },
                            card,
                        );
                    }
                }
            }

            match (&last.typ, &card.typ) {
                (CardType::Skip, CardType::Skip) => {
                    return (Effect::Skip, card);
                }
                (CardType::DrawTwo, CardType::DrawTwo) => {
                    return (Effect::DrawTwo, card);
                }
                (CardType::Reverse, CardType::Reverse) => {
                    return (Effect::Reverse, card);
                }
                (CardType::Wild, CardType::Wild) => {
                    return (Effect::Wild, card);
                }
                (CardType::WildDrawFour, CardType::WildDrawFour) => {
                    return (Effect::WildDrawFour, card);
                }
                _ => (),
            }

            (Effect::Wrong, card)
        } else {
            (
                match &card.typ {
                    CardType::Skip => Effect::Skip,
                    CardType::DrawTwo => Effect::DrawTwo,
                    CardType::Reverse => Effect::Reverse,
                    CardType::Wild => Effect::Wild,
                    CardType::WildDrawFour => Effect::WildDrawFour,
                    CardType::Number(_) => Effect::Nothing,
                },
                card,
            )
        }
    }

    fn put(&mut self, card: Card) {
        self.table.push_back(card);
    }

    fn next_turn(&mut self) -> usize {
        self.turn += 1;
        self.turn %= self.hands.len();
        let turn = self.turn;

        if self.hands[turn].1.is_empty() {
            self.next_turn()
        } else {
            turn
        }
    }

    fn reverse(&mut self) {
        self.hands.reverse();
    }

    fn take_effect(&mut self, effect: Effect, assign_color: Option<Color>) {
        match effect {
            Effect::Nothing => (),
            Effect::Skip => {
                self.next_turn();
            }
            Effect::Reverse => self.reverse(),
            Effect::DrawTwo => {
                self.hands[self.turn].1.append(&mut self.deck.drain(0..2).collect());
            }
            Effect::Wild => {
                let last = self.table.back_mut().unwrap();
                last.color = assign_color;
            }
            Effect::WildDrawFour => {
                self.hands[self.turn].1.append(&mut self.deck.drain(0..4).collect());
                let last = self.table.back_mut().unwrap();
                last.color = assign_color;
            }
            Effect::Wrong => unreachable!("Don't"),
        }
    }

    fn should_ask_for_color(&self) -> bool {
        self.deck.back().unwrap().color.is_none()
    }

    fn current_player_id(&self) -> usize {
        self.hands[self.turn].0
    }

    fn did_win(&self, player_id: usize) -> bool {
        self.hands[player_id].1.is_empty()
    }

    fn mark_win(&mut self) {
        self.wins += 1;
    }

    fn is_ended(&mut self) -> bool {
        self.wins + 1 >= self.hands.len()
    }
}

struct MessageManager {
    messages: Vec<Vec<ChatMessage>>,
}

impl MessageManager {
    fn new(players: usize) -> Self {
        Self {
            messages: (0..players)
                .into_iter()
                .map(|_| vec![ChatMessage::system(UNO_PROMPT, None)])
                .collect(),
        }
    }

    fn add(&mut self, player_id: usize, message: ChatMessage) {
        self.messages[player_id].push(message);
    }

    fn global_add(&mut self, message: ChatMessage) {
        for player in &mut self.messages {
            player.push(message.clone());
        }
    }

    fn get(&self, player_id: usize) -> &Vec<ChatMessage> {
        &self.messages[player_id]
    }
}

async fn ask_for_color(groq: &Groq, manager: &mut MessageManager, id: usize) -> Result<Color> {
    tracing::info!("Asking Player {id} to pick a color");

    manager.add(
        id,
        ChatMessage::user(
            "[GAME] Pick a color (red/green/blue/yellow, no other text allowed, lower case)",
            None
        )
    );
    let ccompl = groq.create_chat_completion(
        ChatCompletionRequest::builder().model(MODEL).messages(&manager.get(id)).build()
    ).await?;
    let mut cchoice = ccompl.choices.unwrap();
    let cchoice = cchoice.pop().unwrap();
    let ctext = cchoice.message.content.get_text();
    let color = Color::from(ctext.trim().to_lowercase().as_str());

    tracing::info!("Player {id} chose color: {color:?}");

    Ok(color)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // TODO: Remove test key lmfao
    let groq = Groq::new(TOKEN.to_string());
    let mut manager = MessageManager::new(3);

    let mut game = UnoGame::new(3);

    let first = game.deck.pop_front().unwrap();
    let (effect, card) = game.get_effect_after_put(first);
    tracing::info!("First card: {card:?}, effect: {effect:?}");

    let (color, description) = {
        if game.should_ask_for_color() {
            let id = game.current_player_id();
            let color = ask_for_color(&groq, &mut manager, id).await?;
            let s = format!(
                "[GAME] First card: {:?}, takes effect on player {}: {}\nPlayer {} chose color {:?}",
                card,
                effect.as_str(),
                id,
                id,
                &color
            );
            (Some(color), s)
        } else {
            let id = game.current_player_id();
            (
                None,
                format!(
                    "[GAME] First card: {:?}, takes effect on player {}: {}",
                    card,
                    id,
                    effect.as_str()
                ),
            )
        }
    };
    manager.global_add(ChatMessage::user(description, None));

    game.put(card);
    game.take_effect(effect, color);

    while !game.is_ended() {
        let id = game.current_player_id();
        let cards = game.view(id);

        tracing::info!("Player {id}'s turn, cards: {cards:?}");

        let s = cards
            .iter()
            .enumerate()
            .map(|(index, item)| format!("{} -> {:?}", index, item))
            .collect::<Vec<_>>()
            .join("\n");
        tracing::info!("Prompt:\n{s}");
        manager.add(
            id,
            ChatMessage::user(
                GAMEPLAY_PROMPT.replace("{cards}", &s)
                    .replace("{n_players}", &(game.hands.len() - game.wins).to_string())
                    .replace("{player_id}", &id.to_string()),
                None
            )
        );

        let compl = groq.create_chat_completion(
            ChatCompletionRequest::builder().model(MODEL).messages(&manager.get(id)).build()
        ).await?;
        let mut choice = compl.choices.unwrap();
        let choice = choice.pop().unwrap();
        let content = choice.message.content;
        let text = content.get_text();
        tracing::info!("Player {id} said:\n{text}\n");

        let last_line = text.lines().last().unwrap().trim();
        if last_line.to_lowercase() == "draw" {
            tracing::info!("Player {id} chose to draw a card!");
            let card = game.deck.pop_front().unwrap();
            tracing::info!("...... card: {card:?}");
            manager.global_add(ChatMessage::user(format!("Player {id} drew a card."), None));
            manager.add(id, ChatMessage::user(format!("Card drew: {card:?}"), None));
            game.next_turn();
        } else {
            let card_n = last_line.parse::<usize>()?;

            let target = game.hands[id].1.remove(card_n).unwrap();
            let target_s = format!("{target:?}");
            tracing::info!("Player {id} chose card index {card_n} -> {target_s}");

            let (effect, target) = game.get_effect_after_put(target);

            if let Effect::Wrong = &effect {
                tracing::warn!("...... chose the wrong card (invalid effect)");
                game.hands[id].1.insert(card_n, target);
                manager.add(
                    id,
                    ChatMessage::user(
                        "[GAME] INVALID PLAY! The card you placed is not valid.",
                        None
                    )
                );
            } else {
                tracing::info!("...... effect: {effect:?}");

                let effect_s = effect.as_str().to_string();
                let color = {
                    if game.should_ask_for_color() {
                        Some(ask_for_color(&groq, &mut manager, id).await?)
                    } else {
                        None
                    }
                };
                game.take_effect(effect, color);

                let next = game.next_turn();
                manager.global_add(
                    ChatMessage::user(
                        format!(
                            "[GAME] Player {} put the card {}\nEffect took on player {}: {}\nNext: Player {}",
                            id,
                            target_s,
                            next,
                            effect_s,
                            &game.hands[next].0
                        ),
                        None
                    )
                );
            }
        }

        std::io::stdin().read_line(&mut String::new())?;
    }

    Ok(())
}
