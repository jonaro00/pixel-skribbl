use components::canvas::Canvas;
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
    let style = use_style!(
        r#"
        display: flex;
        justify-content: center;
        max-width: 1000px;
        margin: auto;
    "#
    );
    html! {
        <>
            <Global css={glob_style}/>
            <div class={style}>
                <Canvas />
            </div>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}

mod components {
    pub mod canvas {
        use super::pixel::{Color, Pixel};
        use strum::IntoEnumIterator;
        use stylist::yew::use_style;
        use yew::prelude::*;
        #[function_component(Canvas)]
        pub fn canvas() -> Html {
            let width: usize = 12;
            let height: usize = 12;

            let selected_color = use_state(|| Color::Black);

            let grid = (0..height)
                .map(|_| {
                    (0..width)
                        .map(|_| {
                            html! {
                                <Pixel selected_color={*selected_color} />
                            }
                        })
                        .collect::<Html>()
                })
                .collect::<Html>();

            let style = use_style!(
                r#"

            "#
            );
            let canvas_style = use_style!(
                r#"
                display: grid;
                grid-template-columns: repeat(${width}, 1fr);
                grid-template-rows: repeat(${height}, 1fr);
                width: 0;
            "#,
                width = width,
                height = height,
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
                }
                > .selectColor.selected {
                    border: 2px dashed black;
                    box-shadow: inset 0 0 9px 5px #ffffff80;
                }
            "#
            );
            html! {
                <div class={style}>
                    <div class={classes!("canvas", canvas_style)}>{grid}</div>
                    <div class={classes!("controls", controls_style)}>
                        { html! {
                            Color::iter().map(|c| {
                                let selected = match *selected_color == c {
                                    true => Some("selected"),
                                    _ => None,
                                };
                                let onclick = {
                                    let selected_color = selected_color.clone();
                                    Callback::from(move |_| selected_color.set(c))
                                };
                                html! {
                                    <div {onclick} class={classes!("selectColor", c.to_string().to_ascii_lowercase(), selected)}></div>
                                }
                            }).collect::<Html>()
                        }}
                        <div class="selectColor white">{ "Clear" }</div>
                    </div>
                </div>
            }
        }
    }
    pub mod pixel {
        use strum::{Display, EnumIter};
        use stylist::yew::use_style;
        use yew::prelude::*;

        #[derive(Debug, PartialEq, Clone, Copy, EnumIter, Display)]
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
        #[derive(Properties, PartialEq)]
        pub struct PixelProps {
            pub selected_color: Color,
        }
        #[function_component(Pixel)]
        pub fn pixel(PixelProps { selected_color }: &PixelProps) -> Html {
            let color = use_state_eq(|| Color::default());

            let onmousedown = {
                let color = color.clone();
                let selected_color = selected_color.clone();
                Callback::from(move |_| color.set(selected_color.clone()))
            };

            let style = use_style!(
                r#"
                width: 40px;
                height: 40px;
                border: 1px solid #00000044;
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
