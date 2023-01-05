use components::{canvas::Canvas, chat::Chat, navbar::NavBar};
use gloo_net::http::Request;
use stylist::{
    css,
    yew::{use_style, Global},
};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use common::Player;

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
        max-width: 1000px;
        margin: auto;
    "#
    );
    let canvas_cont_style = use_style!(
        r#"
        display: flex;
        justify-content: center;
        gap: 10px;
    "#
    );
    let player = use_state(|| None as Option<Player>);
    let _ = use_effect_with_deps({
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
    ());
    html! {
        <>
            <Global css={glob_style}/>
            <ContextProvider<Option<Player>> context={(*player).clone()}>
                <div class={wrapper_style}>
                    <NavBar />
                    <div class={canvas_cont_style}>
                        <Canvas />
                        <Chat />
                    </div>
                </div>
            </ContextProvider<Option<Player>>>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}

mod components {
    pub mod navbar {
        use common::{LoginPost, Player};
        use gloo_net::http::Request;
        use stylist::yew::use_style;
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::spawn_local;
        use web_sys::HtmlInputElement;
        use yew::prelude::*;
        #[derive(PartialEq, Properties)]
        pub struct NavBarProps {}

        #[function_component]
        pub fn NavBar(props: &NavBarProps) -> Html {
            let NavBarProps {} = props;
            let player = use_context::<Option<Player>>().unwrap();
            let style = use_style!(
                r#"
                display: flex;
                flex-wrap: wrap;
                gap: 5px;
                background-color: #6e7eef5e;
                color: #eee;
                padding: 10px;
                border-radius: 10px;
                a {
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
                    <a href="/">{"Draw"}</a>
                    <a href="/gallery">{"Gallery"}</a>
                    <div style="flex-grow: 1;"></div>
                    {
                        if let Some(p) = player {
                            html! { &format!("Logged in as {}", p.username) }
                        } else {
                            html! { <LoginForm /> }
                        }
                    }
                    <a href="/api/logout">{"Log out"}</a>
                </div>
            }
        }
        #[derive(PartialEq, Properties)]
        pub struct LoginFormProps {}
        #[function_component]
        pub fn LoginForm(props: &LoginFormProps) -> Html {
            let LoginFormProps {} = props;
            let username = use_state(String::new);
            let onchangeu = {
                let username = username.clone();
                Callback::from(move |e: Event| {
                    let username = username.clone();
                    username.set(
                        e.target().unwrap().unchecked_into::<HtmlInputElement>().value()
                    );
                }
            )};
            let password = use_state(String::new);
            let onchangep = {
                let password = password.clone();
                Callback::from(move |e: Event| {
                    let password = password.clone();
                    password.set(
                        e.target().unwrap().unchecked_into::<HtmlInputElement>().value()
                    );
                }
            )};
            let onsubmit = {
                let username = username.clone();
                let password = password.clone();
                Callback::from(move |e: SubmitEvent| {
                    e.prevent_default();
                    let username = username.clone();
                    let password = password.clone();
                    spawn_local(async move {
                        Request::post("/api/login")
                        .json(&LoginPost { username: (*username).clone(), password: (*password).clone() })
                        .unwrap()
                        .send()
                        .await
                        .unwrap();
                    });
                    web_sys::window().unwrap().location().reload().unwrap();
                } )}
            ;
            html! {
                <form {onsubmit}>
                    <input type="text" value={(*username).clone()} onchange={onchangeu} />
                    <input type="password" value={(*password).clone()} onchange={onchangep} />
                    <input type="submit" value="Log in" />
                </form>
            }
        }
    }
    pub mod canvas {
        use super::pixel::Pixel;
        use common::{Color, DrawCanvas, GameState, SetPixelPost};
        use futures::StreamExt;
        use gloo_net::{websocket::{futures::WebSocket, Message}, http::Request};
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

            let prompt = use_state(|| "".to_string());

            let selected_color = use_state(|| Color::Black);

            let _ = use_effect_with_deps(
                {
                    let grid = grid.clone();
                    let prompt = prompt.clone();
                    |_| {
                        let host = web_sys::window().unwrap().location().host().unwrap();
                        let ws = WebSocket::open(&format!("ws://{host}/ws/canvas")).unwrap();
                        let (mut _write, mut read) = ws.split();
                        spawn_local(async move {
                            while let Some(Ok(Message::Text(msg))) = read.next().await {
                                console::log_1(&format!("Received on canvas {:?}", msg).into());
                                let gs: GameState = serde_json::from_str(&msg).unwrap();
                                grid.set(gs.canvas.grid);
                                prompt.set(gs.prompt);
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
                        {prompt.to_ascii_lowercase()}
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
                                        // let grid = grid.clone();
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
                width: 40px;
                height: 40px;
                border: 1px solid #00000022;
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
        use common::ChatMessage;
        use futures::StreamExt;
        use gloo_net::{http::Request, websocket::{futures::WebSocket, Message}};
        use stylist::yew::use_style;
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::spawn_local;
        use web_sys::{HtmlInputElement, console, };
        use yew::prelude::*;
        #[derive(PartialEq, Properties)]
        pub struct ChatProps {}
        #[function_component(Chat)]
        pub fn chat(props: &ChatProps) -> Html {
            let ChatProps {} = props;
            let messages = use_mut_ref(|| BoundedVecDeque::<ChatMessage>::new(50));
            let messages_update = use_force_update();
            let text = use_state(String::new);
            let onchange = {
                let text = text.clone();
                Callback::from(move |e: Event| {
                    let text = text.clone();
                    text.set(
                        e.target().unwrap().unchecked_into::<HtmlInputElement>().value()
                    );
                }
                )};
            let onsubmit = {
                let text = text.clone();
                Callback::from(move |e: SubmitEvent| {
                    e.prevent_default();
                    let text = text.clone();
                    if !text.is_empty() {
                        spawn_local(async move {
                            Request::post("/api/chat")
                            .json(&ChatMessage { username: "".into(), text: (*text).clone() })
                            .unwrap()
                            .send()
                            .await
                            .unwrap();
                        });
                    }
                } )}
            ;
            let _ = use_effect_with_deps(
                {
                    let text = text.clone();
                    let messages = messages.clone();
                    |_| {
                        let host = web_sys::window().unwrap().location().host().unwrap();
                        let ws = WebSocket::open(&format!("ws://{host}/ws/chat")).unwrap();
                        let (mut _write, mut read) = ws.split();
                        spawn_local(async move {
                        let text = text.clone();
                            while let Some(Ok(Message::Text(msg))) = read.next().await {
                                console::log_1(&format!("Received on Chat {:?}", msg).into());
                                let cm: ChatMessage = serde_json::from_str(&msg).unwrap();
                                (*messages).borrow_mut().push_back(cm);
                                messages_update.force_update();
                                text.set(String::new());
                            }
                            console::log_1(&"Chat WebSocket Closed".into());
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
                gap: 10px;
                max-width: 245px;
                max-height: 596px;
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
            "#
            );
            html! {
                <div class={style}>
                    <div>{"Users online:"}</div>
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
