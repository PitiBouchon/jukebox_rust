use entity::video::Model as Video;
use yew::prelude::*;

pub enum PlayListMsg {
    Load(Vec<Video>),
    Search(String),
    List(Vec<Video>),
    Remove(usize, String), // Index and id of the video
    Add(Video),
    Play,
    Pause,
    Next,
    SetVolume(f64),
}

#[derive(PartialEq, Clone)]
pub enum PlaylistAction {
    Add(Callback<Video>),
    Remove(Callback<(usize, String)>),
}

#[derive(Properties, PartialEq)]
pub struct PlaylistProp {
    pub id: String,
    pub playlist: Vec<Video>,
    pub callback: PlaylistAction,
}

#[function_component(Playlist)]
pub fn playlist(props: &PlaylistProp) -> Html {
    html! {
        <ul id={ props.id.clone() }>
            {
                props.playlist.clone().iter().enumerate().map(|(i, v)| html! {
                    <li id={ v.id.clone() }>
                        <div>
                            <p>
                                { "Title : "}{ v.title.clone() }{ v.id.clone() }
                            </p>
                            <img src={ v.thumbnail.clone() } width=600 height=400 />
                            <Button info={ v.clone() } callback={ props.callback.clone() } index={ i } />
                        </div>
                    </li>
                }).collect::<Html>()
            }
        </ul>
    }
}

#[derive(Properties, PartialEq)]
pub struct ButtonProp {
    pub info: Video,
    pub callback: PlaylistAction,
    pub index: usize,
}

#[function_component(Button)]
fn button(props: &ButtonProp) -> Html {
    let info = props.info.clone();
    let index = props.index;
    let (callback, text) = match props.callback.clone() {
        PlaylistAction::Add(cb) => (
            Callback::from(move |_| cb.clone().emit(info.clone())),
            "Add",
        ),
        PlaylistAction::Remove(cb) => (
            Callback::from(move |_| cb.clone().emit((index, info.id.clone()))),
            "Remove",
        ),
    };
    html! {<button onclick={ callback.clone() }>{ text }</button>}
}
