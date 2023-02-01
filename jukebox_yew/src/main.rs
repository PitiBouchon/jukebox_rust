use futures::{SinkExt, StreamExt};
use futures::channel::mpsc::Sender;
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message};
use my_youtube_extractor::youtube_info::YtVideoPageInfo;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, window};
use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/index")]
    Home,
    // #[at("/test")]
    // HelloServer,
}

fn switch(routes: Route) -> Html {
    log::info!("Routing");
    match routes {
        Route::Home => html! { <PlayListHtml /> },
        // Route::HelloServer => html! { <HelloServer /> },
    }
}

pub struct PlayListHtml {
    pub playlist: Vec<YtVideoPageInfo>,
    pub send: Sender<String>,
}

pub enum PlayListMsg {
    Set(Vec<YtVideoPageInfo>),
    Send(String),
}

impl Component for PlayListHtml {
    type Message = PlayListMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future(async {
            let resp = Request::get("/api/playlist").send().await.unwrap();
            let playlist_res = serde_json::from_str::<Vec<YtVideoPageInfo>>(&resp.text().await.unwrap()).unwrap();
            PlayListMsg::Set(playlist_res)
        });

        let ws = WebSocket::open("ws://127.0.0.1:4000/websocket").unwrap();

        let (mut write_ws, mut read_ws) = ws.split();
        let (in_tx, mut in_rx) = futures::channel::mpsc::channel::<String>(1000);

        spawn_local(async move {
            while let Some(s) = in_rx.next().await {
                log::debug!("Send to WebSocket: {}", s);
                write_ws.send(Message::Text(s)).await.unwrap();
            }
        });

        let link = ctx.link().clone();

        spawn_local(async move {
            while let Some(Ok(msg)) = read_ws.next().await {
                log::debug!("Receive from WebSocket: {:?}", msg);
                link.send_message(PlayListMsg::Set(vec![]));
            }
            log::info!("WebSocket Closed")
        });

        Self {
            playlist: vec![],
            send: in_tx,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            PlayListMsg::Set(v) => {
                self.playlist = v;
                true
            },
            PlayListMsg::Send(data) => {
                if let Err(err) = self.send.try_send(data) {
                    log::error!("Can't send data to MPSC channel: {err}");
                }
                false
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let cb_send_msg = ctx.link().callback(PlayListMsg::Send);

        let cb_search = Callback::from(move |ev: SubmitEvent| {
            ev.prevent_default(); // Prevent default redirection

            log::debug!("Search : {:?}", ev);
            if let Some(window) = window() {
                if let Some(document) = window.document() {
                    if let Some(input_elm) = document.get_element_by_id("search") {
                        if let Ok(search_elm) = input_elm.dyn_into::<HtmlInputElement>() {
                            cb_send_msg.emit(search_elm.value());
                        }
                    }
                }
            }
        });

        html! {
            <main>
                <form onsubmit={ cb_search }>
                    <input type="search" id="search" name="search" placeholder="Search..." minlength=2/>
                </form>
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

// #[function_component(Index)]
// fn index() -> Html {
//     let data: UseStateHandle<Vec<YtVideoPageInfo>> = use_state(std::vec::Vec::new);
//
//     {
//         let data = data.clone();
//     //     use_effect(move || {
//             spawn_local(async move {
//                 log::info!("Trying to get something");
//                 let resp = Request::get("/api/playlist").send().await.unwrap();
//                 let playlist_res = serde_json::from_str::<Vec<YtVideoPageInfo>>(&resp.text().await.unwrap()).unwrap();
//                 log::info!("Test: {:?}", playlist_res);
//                 data.set(playlist_res);
//             });
//     //
//     //         || {}
//     //     });
//     }
//
//     // log::info!("5");
//     if let Some(info) = data.get(0) {
//         html! {
//             <main>
//                 <h2>{"Playlist :"}</h2>
//                 <ul id="playlist">
//                     <VideoYt info={ info.clone() } />
//                 </ul>
//                 <h2>{ "Searched :" }</h2>
//                 <ul id="search_list">
//                 </ul>
//             </main>
//         }
//     }
//     else {
//         // log::info!("6");
//         html! {
//             <main>
//                 <h2>{"Playlist :"}</h2>
//                 <ul id="playlist">
//                 </ul>
//                 <h2>{ "Searched :" }</h2>
//                 <ul id="search_list">
//                 </ul>
//             </main>
//         }
//     }
// }

// #[function_component(HelloServer)]
// fn hello_server() -> Html {
//     html! { <main><h2>{"TEST"}</h2></main> }
// }

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
    yew::Renderer::<App>::new().render();
}
