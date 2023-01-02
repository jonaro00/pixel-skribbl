use common::chat::{chat_client::ChatClient, ChatMessage};
use components::{canvas::Canvas, navbar::NavBar};
use stylist::{
    css,
    yew::{use_style, Global},
};
use yew::prelude::*;

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
    "#
    );
    html! {
        <>
            <Global css={glob_style}/>
            <div class={wrapper_style}>
                <NavBar />
                <div class={canvas_cont_style}>
                    <Canvas />
                </div>
            </div>
        </>
    }
}

fn main() {
    let mut client = ChatClient::connect("http://localhost:3000/").await.unwrap();
    let request = tonic::Request::new(
        ChatMessage {
            user: "John".into(),
            message: "Hello".into(),
        }
    )
    yew::Renderer::<App>::new().render();
}

mod components {
    pub mod navbar {
        use stylist::yew::use_style;
        use yew::prelude::*;
        #[derive(PartialEq, Properties)]
        pub struct NavBarProps {}

        #[function_component]
        pub fn NavBar(props: &NavBarProps) -> Html {
            let NavBarProps {} = props;
            let style = use_style!(
                r#"
                display: flex;
                flex-wrap: wrap;
                gap: 5px;
                background-color: #6e7eef5e;
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
                </div>
            }
        }
    }
    pub mod canvas {
        use super::pixel::Pixel;
        use common::{Color, DrawCanvas, GameState};
        use futures::StreamExt;
        use gloo_net::websocket::{futures::WebSocket, Message};
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
                        let ws = WebSocket::open(&format!("ws://{host}/ws")).unwrap();
                        let (mut _write, mut read) = ws.split();
                        spawn_local(async move {
                            while let Some(Ok(Message::Text(msg))) = read.next().await {
                                console::log_1(&format!("Received {:?}", msg).into());
                                let gs: GameState = serde_json::from_str(&msg).unwrap();
                                grid.set(gs.canvas.grid);
                                prompt.set(gs.prompt);
                            }
                            console::log_1(&"WebSocket Closed".into());
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
                    <div class={classes!("prompt", prompt_style)}>{prompt.to_ascii_lowercase()}</div>
                    <div class={classes!("canvas", canvas_style)}>{
                        (0..*height)
                        .map(|y| {
                            (0..*width)
                                .map(|x| {
                                    let pos = y * *height + x;
                                    let onclick = {
                                        let grid = grid.clone();
                                        let selected_color = selected_color.clone();
                                        Callback::from(move |_| {
                                            let mut v = (*grid).clone();
                                            if let Some(elem) = v.get_mut(pos) {
                                                *elem = *selected_color;
                                            }
                                            grid.set(v);
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
                            let grid = grid.clone();
                            Callback::from(move |_| {
                                grid.set(vec![Color::default(); *width * *height])
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
}
