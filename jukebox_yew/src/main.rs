use std::process::id;
use futures::TryFutureExt;
// use gloo::utils::document;
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message};
use my_youtube_extractor::youtube_info::YtVideoPageInfo;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/index")]
    Home,
    #[at("/test")]
    HelloServer,
}

fn switch(routes: Route) -> Html {
    log::info!("Routing");
    match routes {
        Route::Home => html! { <PlayListHtml /> },
        Route::HelloServer => html! { <HelloServer /> },
    }
}

pub struct PlayListHtml {
    pub playlist: Vec<YtVideoPageInfo>
}

pub enum PlayListMsg {
    Set(Vec<YtVideoPageInfo>),
}

impl Component for PlayListHtml {
    type Message = PlayListMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future(async {
            log::info!("Trying to get something");
            let resp = Request::get("/api/playlist").send().await.unwrap();
            let playlist_res = serde_json::from_str::<Vec<YtVideoPageInfo>>(&resp.text().await.unwrap()).unwrap();
            log::info!("Test: {:?}", playlist_res);
            PlayListMsg::Set(playlist_res)
        });
        Self {
            playlist: vec![],
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            PlayListMsg::Set(v) => self.playlist = v,
        }

        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <main>
                <h2>{"Playlist :"}</h2>
                <ul id="playlist">
                    { self.playlist.iter().map(|v| html! { <VideoYt info={ v.clone() } /> }).collect::<Html>() }
                </ul>
                <h2>{ "Searched :" }</h2>
                <ul id="search_list">
                </ul>
            </main>
        }
    }
}

#[function_component(Index)]
fn index() -> Html {
    let data: UseStateHandle<Vec<YtVideoPageInfo>> = use_state(std::vec::Vec::new);

    {
        let data = data.clone();
    //     use_effect(move || {
            spawn_local(async move {
                log::info!("Trying to get something");
                let resp = Request::get("/api/playlist").send().await.unwrap();
                let playlist_res = serde_json::from_str::<Vec<YtVideoPageInfo>>(&resp.text().await.unwrap()).unwrap();
                log::info!("Test: {:?}", playlist_res);
                data.set(playlist_res);
            });
    //
    //         || {}
    //     });
    }

    // log::info!("5");
    if let Some(info) = data.get(0) {
        html! {
            <main>
                <h2>{"Playlist :"}</h2>
                <ul id="playlist">
                    <VideoYt info={ info.clone() } />
                </ul>
                <h2>{ "Searched :" }</h2>
                <ul id="search_list">
                </ul>
            </main>
        }
    }
    else {
        // log::info!("6");
        html! {
            <main>
                <h2>{"Playlist :"}</h2>
                <ul id="playlist">
                </ul>
                <h2>{ "Searched :" }</h2>
                <ul id="search_list">
                </ul>
            </main>
        }
    }
}

#[function_component(HelloServer)]
fn hello_server() -> Html {
    html! { <main><h2>{"TEST"}</h2></main> }
}

#[derive(Properties, PartialEq)]
pub struct VideoProps {
    pub info: YtVideoPageInfo
}

#[function_component(VideoYt)]
fn video(video_prop: &VideoProps) -> Html {
    html! {
        <li id={ video_prop.info.id.clone() }>
            <div>
                <p>
                    { "Title : "}{ video_prop.info.title.clone() }{ video_prop.info.id.clone() }
                </p>
                <img src={ video_prop.info.thumbnail.clone() } width=600 height=400 />
                <button>
                    { "Remove" }
                </button>
            </div>
        </li>
    }
}

#[function_component(App)]
fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} />
        </BrowserRouter>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    log::info!("??");
    yew::Renderer::<App>::new().render();
}
