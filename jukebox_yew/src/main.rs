use futures::{SinkExt, StreamExt};
use futures::channel::mpsc::Sender;
use gloo::net::http::Request;
use gloo::net::websocket::{futures::WebSocket, Message};
use my_youtube_extractor::youtube_info::YtVideoPageInfo;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, window};
use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen::JsCast;
use jukebox_rust::{NetDataAxum, NetDataYew};

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
    pub search_videos: Vec<YtVideoPageInfo>,
    pub send: Sender<NetDataYew>,
}

pub enum PlayListMsg {
    RemoveSend(String),
    AddSend(YtVideoPageInfo),
    SearchSend(String),
    SetGet(Vec<YtVideoPageInfo>),
    SearchGet(Vec<YtVideoPageInfo>),
    RemoveGet(String),
}

impl Component for PlayListHtml {
    type Message = PlayListMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future(async {
            let resp = Request::get("/api/playlist").send().await.unwrap();
            let playlist_res = serde_json::from_str::<Vec<YtVideoPageInfo>>(&resp.text().await.unwrap()).unwrap();
            PlayListMsg::SetGet(playlist_res)
        });

        let ws = WebSocket::open("ws://127.0.0.1:4000/websocket").unwrap();

        let (mut write_ws, mut read_ws) = ws.split();
        let (in_tx, mut in_rx) = futures::channel::mpsc::channel::<NetDataYew>(1000);

        spawn_local(async move {
            while let Some(data) = in_rx.next().await {
                log::debug!("Send to WebSocket");
                match data.encode_yew_message() {
                    Ok(msg) => write_ws.send(msg).await.unwrap(),
                    Err(err) => log::error!("Error when encoding NetDataYew: {err}"),
                }
            }
        });

        let link = ctx.link().clone();

        spawn_local(async move {
            while let Some(Ok(msg)) = read_ws.next().await {
                log::debug!("Receive from WebSocket");
                match msg {
                    Message::Bytes(data_encoded) => {
                        match NetDataAxum::decode_message(data_encoded.as_slice()) {
                            Ok(data) => {
                                match data {
                                    NetDataAxum::Remove(video_id) => {
                                        log::info!("Remove video");
                                        link.send_message(PlayListMsg::RemoveGet(video_id));
                                    },
                                    NetDataAxum::Add(_video) => log::info!("Add video"),
                                    NetDataAxum::Search(search_videos) => {
                                        log::info!("Search videos received");
                                        link.send_message(PlayListMsg::SearchGet(search_videos));
                                    }
                                }
                            }
                            Err(err) => log::error!("Error parsing data {err}"),
                        }
                    }
                    _ => log::error!("Unwanted data received"),
                }
            }
            log::info!("WebSocket Closed")
        });

        Self {
            playlist: vec![],
            search_videos: vec![],
            send: in_tx,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            PlayListMsg::SetGet(v) => {
                self.playlist = v;
                true
            },
            PlayListMsg::SearchGet(v) => {
                self.search_videos = v;
                true
            }
            PlayListMsg::SearchSend(data) => {
                if let Err(err) = self.send.try_send(NetDataYew::Search(data)) {
                    log::error!("Can't send data to MPSC channel: {err}");
                }
                false
            },
            PlayListMsg::RemoveSend(video_id) => {
                if let Err(err) = self.send.try_send(NetDataYew::Remove(video_id)) {
                    log::error!("Can't send data to MPSC channel: {err}");
                }
                false
            },
            PlayListMsg::RemoveGet(video_id) => {
                log::debug!("Remove get");
                if let Some(window) = window() {
                    if let Some(document) = window.document() {
                        if let Some(video_elm) = document.get_element_by_id(&video_id) {
                            log::info!("Remove video element");
                            video_elm.remove();
                        }
                    }
                }
                false
            },
            PlayListMsg::AddSend(video) => {
                if let Err(err) = self.send.try_send(NetDataYew::Add(video)) {
                    log::error!("Can't send data to MPSC channel: {err}");
                }
                false
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let cb_send_msg = ctx.link().callback(PlayListMsg::SearchSend);
        let cb_remove = ctx.link().callback(PlayListMsg::RemoveSend);
        let cb_add = ctx.link().callback(PlayListMsg::AddSend);

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
                    { self.playlist.iter().map(|v| html! { <VideoYt info={ v.clone() } remove_cb={ cb_remove.clone() } /> }).collect::<Html>() }
                </ul>
                <h2>{ "Searched :" }</h2>
                <ul id="search_list">
                    { self.search_videos.iter().map(|v| html! { <VideoYtSearch info={ v.clone() } add_cb={ cb_add.clone() } /> }).collect::<Html>() }
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
    pub info: YtVideoPageInfo,
    pub remove_cb: Callback<String, ()>
}

#[function_component(VideoYt)]
fn video(video_prop: &VideoProps) -> Html {
    // WTF tous les .clone() ??!!
    let cb_remove = video_prop.remove_cb.clone();
    let video_id = video_prop.info.id.clone();
    let cb_remove2 = Callback::from(move |_| { cb_remove.clone().emit(video_id.clone()) });
    html! {
        <li id={ video_prop.info.id.clone() }>
            <div>
                <p>
                    { "Title : "}{ video_prop.info.title.clone() }{ video_prop.info.id.clone() }
                </p>
                <img src={ video_prop.info.thumbnail.clone() } width=600 height=400 />
                <button onclick={ cb_remove2.clone() }>
                    { "Remove" }
                </button>
            </div>
        </li>
    }
}

#[derive(Properties, PartialEq)]
pub struct VideoSearchProps {
    pub info: YtVideoPageInfo,
    pub add_cb: Callback<YtVideoPageInfo, ()>
}

#[function_component(VideoYtSearch)]
fn video(video_prop: &VideoSearchProps) -> Html {
    let add_cb = video_prop.add_cb.clone();
    let video = video_prop.info.clone();
    let add_cb2 = Callback::from(move |_| {
        add_cb.clone().emit(video.clone());
    });

    html! {
        <li id={ video_prop.info.id.clone() + "_search" }>
            <div>
                <p>
                    { "Title : "}{ video_prop.info.title.clone() }{ video_prop.info.id.clone() }
                </p>
                <img src={ video_prop.info.thumbnail.clone() } width=600 height=400 />
                <button onclick={ add_cb2.clone() } >
                    { "Add" }
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
