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
        Route::Home => html! {
            <main>
                <h2>{"Playlist :"}</h2>
                <ul id="playlist">
                </ul>
                <h2>{ "Searched :" }</h2>
                <ul id="search_list">
                </ul>
            </main>
        },
        Route::HelloServer => html! { <main><h2>{"TEST"}</h2></main> },
    }
}

#[function_component(HelloServer)]
fn hello_server() -> Html {
    html! { <main>{ "Hello Server" }</main> }
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
