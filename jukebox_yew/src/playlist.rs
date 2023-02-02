use my_youtube_extractor::youtube_info::YtVideoPageInfo;
use yew::prelude::*;

pub enum PlayListMsg {
    RemoveSend(String),
    AddSend(YtVideoPageInfo),
    AddGet(YtVideoPageInfo),
    SearchSend(String),
    SetGet(Vec<YtVideoPageInfo>),
    SearchGet(Vec<YtVideoPageInfo>),
    RemoveGet(String),
}

#[derive(PartialEq, Clone)]
pub enum PlaylistAction {
    Add(Callback<YtVideoPageInfo>),
    Remove(Callback<String>)
}

#[derive(Properties, PartialEq)]
pub struct PlaylistProp
{
    pub id: String,
    pub playlist: Vec<YtVideoPageInfo>,
    pub callback: PlaylistAction,
}

#[function_component(Playlist)]
pub fn playlist(props: &PlaylistProp) -> Html 
{
    html! {
        <ul id={ props.id.clone() }>
            {
                props.playlist.clone().iter().map(|v| html! { 
                    <li id={ v.id.clone() }>
                        <div>
                            <p>
                                { "Title : "}{ v.title.clone() }{ v.id.clone() }
                            </p>
                            <img src={ v.thumbnail.clone() } width=600 height=400 />
                            <Button info={ v.clone() } callback={ props.callback.clone() } />
                        </div>
                    </li>
                }).collect::<Html>() 
            }
        </ul>
    }
}

#[derive(Properties, PartialEq)]
pub struct ButtonProp {
    pub info: YtVideoPageInfo,
    pub callback: PlaylistAction,
}

#[function_component(Button)]
fn button(props: &ButtonProp) -> Html 
{
        let info = props.info.clone();
        let (callback, text) = match props.callback.clone() {
            PlaylistAction::Add(cb) => (Callback::from(move |_| {cb.clone().emit(info.clone())}), "Add"),
            PlaylistAction::Remove(cb) => (Callback::from(move |_| {cb.clone().emit(info.id.clone())}), "Remove")
        };
        html! {<button onclick={ callback.clone() }>{ text }</button>}
}