use components::{
    game::Game,
    navbar::{LoginForm, NavBar},
};
use gloo_net::http::Request;
use stylist::{
    css,
    yew::{use_style, Global},
};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

fn main() {
    yew::Renderer::<App>::new().render();
}

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/game/:room_id")]
    Game { room_id: u32 },
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[function_component(App)]
fn app() -> Html {
    let glob_style = css!(
        r#"
        height: 100%;
        background: linear-gradient(300deg, #009 0%, #606 100%);

        *, *::before, *::after {
            box-sizing: border-box;
        }

        .white { background-color: white; }
        .gray { background-color: gray; }
        .black { background-color: black; }
        .red { background-color: red; }
        .orange { background-color: orange; }
        .yellow { background-color: yellow; }
        .lime { background-color: lime; }
        .green { background-color: green; }
        .blue { background-color: blue; }
        .cyan { background-color: cyan; }
        .magenta { background-color: magenta; }
        .purple { background-color: purple; }
    "#
    );
    let wrapper_style = use_style!(
        r#"
        display: flex;
        flex-direction: column;
        gap: 10px;
        max-width: 1100px;
        margin: auto;
    "#
    );
    let player = use_state(|| None as Option<String>);
    use_effect_with_deps(
        {
            let player = player.clone();
            |_| {
                spawn_local(async move {
                    let p = Request::get("/api/player")
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .unwrap();
                    player.set(p);
                });
            }
        },
        (),
    );
    html! {
        <>
            <Global css={glob_style}/>
            <ContextProvider<Option<String>> context={(*player).clone()}>
                <BrowserRouter>
                    <div class={wrapper_style}>
                        <NavBar />
                        <Switch<Route> render={|r| match r {
                            Route::Home => html! { <LoginForm create_lobby={true} /> },
                            Route::Game { room_id } => html! { <Game {room_id} /> },
                            Route::NotFound => html! { "Not found 🤔" },
                        }} />
                    </div>
                </BrowserRouter>
            </ContextProvider<Option<String>>>
        </>
    }
}

mod components {
    pub mod navbar {
        use common::JoinLobbyPost;
        use gloo_net::http::Request;
        use stylist::yew::use_style;
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::spawn_local;
        use web_sys::HtmlInputElement;
        use yew::prelude::*;
        use yew_router::prelude::*;

        use crate::Route;

        #[derive(PartialEq, Properties)]
        pub struct NavBarProps {}
        #[function_component]
        pub fn NavBar(props: &NavBarProps) -> Html {
            let NavBarProps {} = props;
            let player = use_context::<Option<String>>().unwrap();
            let style = use_style!(
                r#"
                display: flex;
                flex-wrap: wrap;
                gap: 5px;
                background-color: #6e7eef5e;
                color: #eee;
                padding: 10px;
                border-radius: 10px;
                & > a, & > form, & > div {
                    background-color: #6e7eef5e;
                    padding: 10px;
                    border-radius: 10px;
                    color: #eee;
                    text-decoration: none;
                }
            "#
            );
            html! {
                <div class={style}>
                    // <a href="/">{"Draw"}</a>
                    // <a href="/gallery">{"Gallery"}</a>
                    <Link<Route> to={Route::Home}>{ "Create Lobby" }</Link<Route>>
                    // <Link<Route> to={Route::Join { room_id: () }}>{ "Gallery" }</Link<Route>>
                    <i style="flex-grow: 1;"></i>
                    {
                        if let Some(p) = player {
                            html! {
                                <>
                                    <div>{&format!("Logged in as {}", p)}</div>
                                    <a href="/api/logout">{"Log out"}</a>
                                </>
                            }
                        } else {
                            html! { <LoginForm create_lobby={false} /> }
                        }
                    }
                </div>
            }
        }
        #[derive(PartialEq, Properties)]
        pub struct LoginFormProps {
            pub create_lobby: bool,
        }
        #[function_component]
        pub fn LoginForm(props: &LoginFormProps) -> Html {
            let LoginFormProps { create_lobby } = props;
            let username = use_state(String::new);
            let onchangeu = {
                let username = username.clone();
                Callback::from(move |e: Event| {
                    let username = username.clone();
                    username.set(
                        e.target()
                            .unwrap()
                            .unchecked_into::<HtmlInputElement>()
                            .value(),
                    );
                })
            };
            // let password = use_state(String::new);
            // let onchangep = {
            //     let password = password.clone();
            //     Callback::from(move |e: Event| {
            //         let password = password.clone();
            //         password.set(
            //             e.target()
            //                 .unwrap()
            //                 .unchecked_into::<HtmlInputElement>()
            //                 .value(),
            //         );
            //     })
            // };
            let onsubmit = {
                let username = username.clone();
                let create_lobby = create_lobby.clone();
                // let password = password.clone();
                Callback::from(move |e: SubmitEvent| {
                    e.prevent_default();
                    let username = username.clone();
                    let create_lobby = create_lobby.clone();
                    // let password = password.clone();
                    spawn_local(async move {
                        let endp = if create_lobby {
                            "/api/create_lobby"
                        } else {
                            "/api/join_lobby"
                        };
                        let resp = Request::post(endp)
                            .json(&JoinLobbyPost {
                                username: (*username).clone(),
                            })
                            .unwrap()
                            .send()
                            .await
                            .unwrap();
                        if create_lobby {
                            let room = resp.text().await.unwrap();
                            web_sys::window()
                                .unwrap()
                                .location()
                                .replace(&format!("/game/{room}"))
                                .unwrap();
                        } else {
                            web_sys::window().unwrap().location().reload().unwrap();
                        }
                    });
                })
            };
            html! {
                <form {onsubmit}>
                    <input type="text" value={(*username).clone()} onchange={onchangeu} />
                    // <input type="password" value={(*password).clone()} onchange={onchangep} />
                    <input type="submit" value="Log in" />
                </form>
            }
        }
    }
    pub mod game {
        use super::{canvas::Canvas, chat::Chat};
        use common::GameInfo;
        use futures::StreamExt;
        use gloo_net::websocket::{futures::WebSocket, Message};
        use stylist::yew::use_style;
        use wasm_bindgen_futures::spawn_local;
        use web_sys::console;
        use yew::prelude::*;

        #[derive(PartialEq, Properties)]
        pub struct GameProps {
            pub room_id: u32,
        }

        #[function_component]
        pub fn Game(props: &GameProps) -> Html {
            let GameProps { room_id } = props;
            let gi = use_state(|| GameInfo {
                // room_id: *room_id,
                ..Default::default()
            });
            use_effect_with_deps(
                {
                    let gi = gi.clone();
                    |_| {
                        let host = web_sys::window().unwrap().location().host().unwrap();
                        let secure =
                            web_sys::window().unwrap().location().protocol().unwrap() == "https:";
                        let ws = WebSocket::open(&format!(
                            "ws{}://{host}/ws/game",
                            if secure { "s" } else { "" }
                        ))
                        .unwrap();
                        let (mut _write, mut read) = ws.split();
                        spawn_local(async move {
                            while let Some(Ok(Message::Text(msg))) = read.next().await {
                                console::log_1(&format!("Received on game {:?}", msg).into());
                                let g: GameInfo = serde_json::from_str(&msg).unwrap();
                                gi.set(g);
                            }
                            console::log_1(&"Game WebSocket Closed".into());
                        });
                        ()
                    }
                },
                (),
            );
            let style = use_style!(
                r#"
                display: flex;
                justify-content: center;
                gap: 10px;

                @media screen and (max-width: 780px) {
                    flex-wrap: wrap;
                }
            "#
            );
            html! {
                <div class={style}>
                    <ContextProvider<GameInfo> context={(*gi).clone()}>
                        <Canvas />
                        <Chat />
                    </ContextProvider<GameInfo>>
                </div>
            }
        }
    }
    pub mod canvas {
        use super::pixel::Pixel;
        use common::{Color, DrawCanvas, GameInfo, SetPixelPost};
        use futures::StreamExt;
        use gloo_net::{
            http::Request,
            websocket::{futures::WebSocket, Message},
        };
        use strum::IntoEnumIterator;
        use stylist::yew::use_style;
        use wasm_bindgen_futures::spawn_local;
        use web_sys::console;
        use yew::prelude::*;

        #[function_component(Canvas)]
        pub fn canvas() -> Html {
            let width = use_state_eq(|| 12usize);
            let height = use_state_eq(|| 12usize);
            let dc = DrawCanvas::default();
            let grid = use_state_eq(|| dc.grid);

            let game_info = use_context::<GameInfo>().unwrap();
            let prompt = game_info.prompt;

            let selected_color = use_state(|| Color::Black);

            use_effect_with_deps(
                {
                    let grid = grid.clone();
                    |_| {
                        let host = web_sys::window().unwrap().location().host().unwrap();
                        let secure =
                            web_sys::window().unwrap().location().protocol().unwrap() == "https:";
                        let ws = WebSocket::open(&format!(
                            "ws{}://{host}/ws/canvas",
                            if secure { "s" } else { "" }
                        ))
                        .unwrap();
                        let (mut _write, mut read) = ws.split();
                        spawn_local(async move {
                            while let Some(Ok(Message::Text(msg))) = read.next().await {
                                console::log_1(&format!("Received on canvas {:?}", msg).into());
                                let c: DrawCanvas = serde_json::from_str(&msg).unwrap();
                                grid.set(c.grid);
                            }
                            console::log_1(&"Canvas WebSocket Closed".into());
                        });
                        ()
                    }
                },
                (),
            );

            let style = use_style!(
                r#"
                display: flex;
                flex-direction: column;
                gap: 5px;
                align-items: center;
            "#
            );
            let prompt_style = use_style!(
                r#"
                background-color: #ffffffcc;
                font-size: 1.5em;
                padding: 5px;
                font-family: monospace;
                letter-spacing: .2em;
            "#
            );
            let canvas_style = use_style!(
                r#"
                display: grid;
                grid-template-columns: repeat(${width}, 1fr);
                grid-template-rows: repeat(${height}, 1fr);
                user-select: none;
                width: 100%;
                max-width: 480px;
                aspect-ratio: 1;
            "#,
                width = *width,
                height = *height,
            );
            let controls_style = use_style!(
                r#"
                display: flex;
                flex-wrap: wrap;
                gap: 5px;
                background-color: #6e7eef5e;
                padding: 10px;
                border-radius: 10px;

                > .selectColor {
                    width: 50px;
                    height: 50px;
                    border-radius: 50%;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    user-select: none;
                    cursor: pointer;
                }
                > .selectColor.selected {
                    border: 2px dashed black;
                    box-shadow: inset 0 0 9px 5px #ffffff80;
                }
            "#
            );
            html! {
                <div class={style}>
                    <div class={classes!("prompt", prompt_style)}>
                        {prompt.clone()}
                        {if prompt.is_empty() { html! { <></> } } else {
                            html! {
                                <sub style={"font-family: sans-serif; font-size: 60%; letter-spacing: normal;"}>
                                    {
                                        prompt.chars().map(|c| (c != ' ') as usize).sum::<usize>()
                                    }
                                </sub>
                            }
                        }}
                    </div>
                    <div class={classes!("canvas", canvas_style)}>{
                        (0..*height)
                        .map(|y| {
                            (0..*width)
                                .map(|x| {
                                    let pos = y * *height + x;
                                    let onclick = {
                                        let selected_color = selected_color.clone();
                                        Callback::from(move |_| {
                                            let selected_color = selected_color.clone();
                                            spawn_local(async move {
                                                Request::post("/api/set_pixel")
                                                    .json(&SetPixelPost { pixel_id: pos, color: *selected_color })
                                                    .unwrap()
                                                    .send()
                                                    .await
                                                    .unwrap();
                                            });
                                        })
                                    };
                                    html! {
                                        <Pixel key={pos} color={grid[pos]} {onclick} />
                                    }
                                })
                                .collect::<Html>()
                        })
                        .collect::<Html>()
                    }
                    </div>
                    <div class={classes!("controls", controls_style)}>
                        {
                            Color::iter().map(|c| {
                                let selected = if *selected_color == c {
                                    Some("selected")} else { None
                                };
                                let onclick = {
                                    let selected_color = selected_color.clone();
                                    Callback::from(move |_| selected_color.set(c))
                                };
                                html! {
                                    <div {onclick} class={classes!("selectColor", c.to_string().to_ascii_lowercase(), selected)}></div>
                                }
                            }).collect::<Html>()
                        }
                        <div onclick={{
                            Callback::from(move |_| {
                                spawn_local(async move {
                                    Request::get("/api/clear_canvas")
                                        .send()
                                        .await
                                        .unwrap();
                                });
                            })
                        }} class="selectColor white">{ "Clear" }</div>
                        <div onclick={{
                            Callback::from(move |_| {
                                spawn_local(async move {
                                    Request::get("/api/gallery/save")
                                        .send()
                                        .await
                                        .unwrap();
                                });
                            })
                        }} class="selectColor white">{ "Save" }</div>
                    </div>
                </div>
            }
        }
    }
    pub mod pixel {
        use stylist::yew::use_style;
        use yew::prelude::*;

        use common::Color;

        #[derive(Properties, PartialEq)]
        pub struct PixelProps {
            pub color: Color,
            pub onclick: Callback<()>,
        }
        #[function_component(Pixel)]
        pub fn pixel(PixelProps { color, onclick }: &PixelProps) -> Html {
            let onmousedown = {
                let onclick = onclick.clone();
                Callback::from(move |_| onclick.emit(()))
            };

            let style = use_style!(
                r#"
                min-width: 10px;
                min-height: 10px;
                border: .5px solid #00000022;
                cursor: crosshair;
            "#
            );

            html! {
                <div
                    class={classes!("pixel", style, format!("{:?}", *color).to_ascii_lowercase())}
                    {onmousedown}
                />
            }
        }
    }
    pub mod chat {
        use bounded_vec_deque::BoundedVecDeque;
        use common::{ChatMessage, GameInfo};
        use futures::StreamExt;
        use gloo_net::{
            http::Request,
            websocket::{futures::WebSocket, Message},
        };
        use stylist::yew::use_style;
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::spawn_local;
        use web_sys::{console, HtmlInputElement};
        use yew::prelude::*;
        #[derive(PartialEq, Properties)]
        pub struct ChatProps {}
        #[function_component(Chat)]
        pub fn chat(props: &ChatProps) -> Html {
            let ChatProps {} = props;
            let messages = use_mut_ref(|| BoundedVecDeque::<ChatMessage>::new(50));
            let messages_update = use_force_update();
            let text = use_state(String::new);
            let game_info = use_context::<GameInfo>().unwrap();
            let players = game_info.players;
            let onchange = {
                let text = text.clone();
                Callback::from(move |e: Event| {
                    let text = text.clone();
                    text.set(
                        e.target()
                            .unwrap()
                            .unchecked_into::<HtmlInputElement>()
                            .value(),
                    );
                })
            };
            let onsubmit = {
                let text = text.clone();
                Callback::from(move |e: SubmitEvent| {
                    e.prevent_default();
                    let text = text.clone();
                    if !text.is_empty() {
                        spawn_local(async move {
                            Request::post("/api/chat")
                                .json(&ChatMessage {
                                    username: "".into(),
                                    text: (*text).clone(),
                                })
                                .unwrap()
                                .send()
                                .await
                                .unwrap();
                            text.set(String::new());
                        });
                    }
                })
            };
            use_effect_with_deps(
                {
                    let messages = messages.clone();
                    |_| {
                        let host = web_sys::window().unwrap().location().host().unwrap();
                        let secure =
                            web_sys::window().unwrap().location().protocol().unwrap() == "https:";
                        let ws = WebSocket::open(&format!(
                            "ws{}://{host}/ws/chat",
                            if secure { "s" } else { "" }
                        ))
                        .unwrap();
                        let (mut _write, mut read) = ws.split();
                        let f = async move {
                            while let Some(Ok(Message::Text(msg))) = read.next().await {
                                console::log_1(&format!("Received on Chat {:?}", msg).into());
                                let cm: ChatMessage = serde_json::from_str(&msg).unwrap();
                                (*messages).borrow_mut().push_back(cm);
                                messages_update.force_update();
                            }
                            console::log_1(&"Chat WebSocket Closed".into());
                        };
                        spawn_local(f);
                        ()
                    }
                },
                (),
            );
            let style = use_style!(
                r#"
                flex: 0 0 245px;
                display: flex;
                flex-direction: column;
                gap: 10px;
                max-height: 651px;
                background-color: #6e7eef5e;
                padding: 10px;
                border-radius: 10px;
                color: #eee;
                word-break: break-word;
            "#
            );
            let chat_style = use_style!(
                r#"
                flex-grow: 1;
                display: flex;
                flex-direction: column;
                justify-content: flex-end;
                gap: 3px;
                width: 100%;
                overflow-y: auto;

                & > * {
                    padding: 3px;
                    border-radius: 3px;
                    background-color: #00000022;
                }
                & > * > b {
                    font-style: italic;
                }
            "#
            );
            html! {
                <div class={style}>
                    <div>
                        {
                            format!(
                                "Users online ({}): {}",
                                players.len(),
                                players
                                    .into_iter()
                                    .map(|p| if p.active { format!("{} (drawing)", p.username) } else { p.username })
                                    .collect::<Vec<String>>()
                                    .join(", "))
                        }
                    </div>
                    <div class={chat_style}>
                        {
                            (*messages)
                                .borrow()
                                .iter()
                                .map(
                                    |msg| html! {
                                        <div><b>{msg.username.clone()}</b>{": "}{msg.text.clone()}</div>
                                    }
                                )
                                .collect::<Html>()
                        }
                    </div>
                    <div>
                        <form {onsubmit}>
                            <input type="text" value={(*text).clone()} {onchange} />
                            <input type="submit" value="Send" />
                        </form>
                    </div>
                </div>
            }
        }
    }
}
