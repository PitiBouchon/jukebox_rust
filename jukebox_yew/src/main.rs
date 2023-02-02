use std::collections::HashMap;

use futures::{SinkExt, StreamExt};
use futures::channel::mpsc::Sender;
use gloo::net::http::Request;
use gloo::net::websocket::{futures::WebSocket, Message};
use my_youtube_extractor::youtube_info::YtVideoPageInfo;
use playlist::{PlayListMsg, PlaylistAction};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, window};
use yew::prelude::*;
use yew_router::prelude::*;
use wasm_bindgen::JsCast;
use jukebox_rust::{NetDataAxum, NetDataYew};

mod playlist;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/index")]
    Home,
}

fn switch(routes: Route) -> Html {
    log::info!("Routing");
    match routes {
        Route::Home => html! { <PlayListHtml /> },
        // Route::HelloServer => html! { <HelloServer /> },
    }
}

pub struct PlayListHtml {
    pub playlist: HashMap<String, YtVideoPageInfo>,
    pub search_videos: Vec<YtVideoPageInfo>,
    pub send: Sender<NetDataYew>,
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
                                    NetDataAxum::Add(video) => {
                                        log::info!("Add video");
                                        link.send_message(PlayListMsg::AddGet(video));
                                    },
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
            playlist: HashMap::new(),
            search_videos: vec![],
            send: in_tx,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            PlayListMsg::SetGet(v) => {
                self.playlist = v.iter().map(|x| (x.id.to_owned(), x.to_owned())).collect();
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
                self.playlist.remove(&video_id);
                true
            },
            PlayListMsg::AddSend(video) => {
                if let Err(err) = self.send.try_send(NetDataYew::Add(video)) {
                    log::error!("Can't send data to MPSC channel: {err}");
                }
                false
            },
            PlayListMsg::AddGet(video) => {
                log::debug!("Add get");
                self.playlist.insert(video.id.to_owned(), video);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let cb_send_msg = ctx.link().callback(PlayListMsg::SearchSend);
        let cb_remove = PlaylistAction::Remove(ctx.link().callback(PlayListMsg::RemoveSend).clone());
        let cb_add = PlaylistAction::Add(ctx.link().callback(PlayListMsg::AddSend).clone());

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
                <playlist::Playlist id={"videos".to_owned()} playlist={ self.playlist.clone().into_values().collect::<Vec<YtVideoPageInfo>>() } callback={ cb_remove } />
                <h2>{ "Searched :" }</h2>
                <playlist::Playlist id={"search".to_owned()} playlist={ self.search_videos.clone() } callback={ cb_add } />
            </main>
        }
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
