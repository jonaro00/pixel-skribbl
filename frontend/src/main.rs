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

        .white {
            background-color: white;
        }
        .black {
            background-color: black;
        }
    "#
    );
    let style = use_style!(
        r#"
        display: flex;
        justify-content: center;
        max-width: 1000px;
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
                display: grid;
                grid-template-columns: repeat(${width}, 1fr);
                grid-template-rows: repeat(${height}, 1fr);
            "#,
                width = width,
                height = height
            );
            html! {
                <>
                    <div class={classes!("canvas", style)}>{grid}</div>
                    <div class="controls">{ html! {
                        Color::iter().map(|c| {
                            html! {
                                <div class={classes!("selectColor", c.to_string().to_ascii_lowercase())}></div>
                            }
                        }).collect::<Html>()
                    }}</div>
                </>
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

            let onclick = {
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
                    {onclick}
                />
            }
        }
    }
}
