use entity::video::Model as Video;
use futures::{SinkExt, StreamExt};
use gloo::net::http::Request;
use gloo::net::websocket::{futures::WebSocket, Message};
use jukebox_rust::NetData;
use playlist::{PlayListMsg, PlaylistAction};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, HtmlInputElement};
use yew::platform::pinned::mpsc::UnboundedSender;
use yew::prelude::*;
use yew_router::prelude::*;

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
    }
}

pub struct PlayListHtml {
    pub playlist: Vec<Video>,
    pub search_videos: Vec<Video>,
    pub send: UnboundedSender<NetData>,
    pub volume: f64,
}

impl Component for PlayListHtml {
    type Message = PlayListMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future(async {
            let resp = Request::get("/api/playlist").send().await.unwrap();
            let playlist_res =
                serde_json::from_str::<Vec<Video>>(&resp.text().await.unwrap()).unwrap();
            PlayListMsg::Load(playlist_res)
        });

        let ws = WebSocket::open("ws://127.0.0.1:4000/websocket").unwrap();

        let (mut write_ws, mut read_ws) = ws.split();
        let (in_tx, mut in_rx) = yew::platform::pinned::mpsc::unbounded::<NetData>();
        //let (in_tx, mut in_rx) = futures::channel::mpsc::channel::<NetData>(1000);

        spawn_local(async move {
            while let Some(data) = in_rx.next().await {
                log::debug!("Send to WebSocket");
                write_ws
                    .send(Message::Bytes(data.encode_message().unwrap()))
                    .await
                    .unwrap();
            }
        });

        let link = ctx.link().clone();

        spawn_local(async move {
            while let Some(Ok(msg)) = read_ws.next().await {
                log::debug!("Receive from WebSocket");
                match msg {
                    Message::Bytes(data_encoded) => {
                        match NetData::decode_message(data_encoded.as_slice()) {
                            Ok(data) => match data {
                                NetData::Remove(index, video_id) => {
                                    log::info!("Remove video");
                                    link.send_message(PlayListMsg::Remove(index, video_id));
                                }
                                NetData::Add(video) => {
                                    log::info!("Add video");
                                    link.send_message(PlayListMsg::Add(video));
                                }
                                NetData::SearchResult(search_videos) => {
                                    log::info!("Search videos received");
                                    link.send_message(PlayListMsg::List(search_videos));
                                }
                                NetData::Next => {
                                    log::info!("Video Passed");
                                    link.send_message(PlayListMsg::Next);
                                }
                                _ => {}
                            },
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
            volume: 100.0,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            PlayListMsg::Load(v) => {
                self.playlist = v;
                true
            }
            PlayListMsg::List(v) => {
                self.search_videos = v;
                true
            }
            PlayListMsg::Search(data) => {
                if let Err(err) = self.send.send_now(NetData::Search(data)) {
                    log::error!("Can't send data to MPSC channel: {err}");
                }
                true
            }
            PlayListMsg::Remove(index, video_id) => {
                if let Some(video) = self.playlist.get(index) {
                    if video.id == video_id {
                        self.playlist.remove(index);
                    } else {
                        log::error!("Trying to remove a video that is not present");
                    }
                }
                true
            }
            PlayListMsg::Add(video) => {
                log::debug!("Add get");
                self.playlist.push(video);
                true
            }
            PlayListMsg::Play => {
                if let Err(err) = self.send.send_now(NetData::Play) {
                    log::error!("Can't send data to MPSC channel: {err}");
                }
                true
            }
            PlayListMsg::Pause => {
                if let Err(err) = self.send.send_now(NetData::Pause) {
                    log::error!("Can't send data to MPSC channel: {err}");
                }
                true
            }
            PlayListMsg::Next => {
                if !self.playlist.is_empty() {
                    self.playlist.remove(0);
                }
                true
            }
            PlayListMsg::SetVolume(volume) => {
                if let Err(err) = self.send.send_now(NetData::SetVolume(volume)) {
                    log::error!("Can't send data to MPSC channel: {err}");
                }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let sender = self.send.clone();
        let cb_remove = PlaylistAction::Remove(Callback::from(
            move |(video_index, video_id): (usize, String)| {
                log::debug!("Removing id: {}", video_id);
                let _ = sender.send_now(NetData::Remove(video_index, video_id));
            },
        ));

        let sender = self.send.clone();
        let cb_add = PlaylistAction::Add(Callback::from(move |video: Video| {
            let _ = sender.send_now(NetData::Add(video));
        }));

        let sender = self.send.clone();
        let cb_play = Callback::from(move |_| {
            let _ = sender.send_now(NetData::Play);
        });

        let sender = self.send.clone();
        let cb_pause = Callback::from(move |_| {
            let _ = sender.send_now(NetData::Pause);
        });

        let sender = self.send.clone();
        let cb_next = Callback::from(move |_| {
            let _ = sender.send_now(NetData::Next);
        });

        let sender = self.send.clone();
        let cb_send_msg = Callback::from(move |search: String| {
            let _ = sender.send_now(NetData::Search(search));
        });
        let cb_search = Callback::from(move |ev: SubmitEvent| {
            ev.prevent_default();
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

        let cb_change_volume = ctx.link().callback(PlayListMsg::SetVolume);
        let oninput = Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target_dyn_into::<HtmlInputElement>() {
                cb_change_volume.clone().emit(input.value_as_number());
            }
        });

        html! {
            <main>
                <form onsubmit={ cb_search }>
                    <input type="search" id="search" name="search" placeholder="Search..." minlength=2/>
                </form>
                <button onclick={ cb_play.clone() }>{ "Play" }</button>
                <button onclick={ cb_pause.clone() }>{ "Pause" }</button>
                <button onclick={ cb_next.clone() }>{ "Next" }</button>
                <input type="range"
                        value={self.volume.to_string()}
                        class="slider__input"
                        min=0 max=100 step=1
                        {oninput}
                />
                <h2>{"Playlist :"}</h2>
                <playlist::Playlist id={"videos"} playlist={ self.playlist.clone() } callbacks={ vec![cb_remove] } />
                <h2>{ "Searched :" }</h2>
                <playlist::Playlist id={"search"} playlist={ self.search_videos.clone() } callbacks={ vec![cb_add] } />
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
