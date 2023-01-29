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
        Route::Home => html! { <Index /> },
        Route::HelloServer => html! { <HelloServer /> },
    }
}

#[function_component(Index)]
fn index() -> Html {
    spawn_local(async move {
        log::info!("Trying to get something");
        let resp = Request::get("127.0.0.1:4000/api/playlist").send().await.unwrap();
        let playlist_res = serde_json::from_str::<Vec<YtVideoPageInfo>>(&resp.text().await.unwrap()).unwrap();
        let playlist_html: Vec<Html> = playlist_res
            .into_iter()
            .map(|video_info| {
                html! {
                <li id={ video_info.id.clone() }>
                    <div>
                        <p>
                            { "Title : "}{ video_info.title } { video_info.id }
                        </p>
                        <img src={ video_info.thumbnail } width=600 height=400 />
                        <button>
                            { "Remove" }
                        </button>
                    </div>
                </li>
            }
            })
            .collect();

        log::info!("Test: {:?}", playlist_html);
    });

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

#[function_component(HelloServer)]
fn hello_server() -> Html {
    html! { <main><h2>{"TEST"}</h2></main> }
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
    yew::Renderer::<App>::new().render();
}
