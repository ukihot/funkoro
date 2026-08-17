use bevy::prelude::*;

mod game;

#[cfg(feature = "dev")]
use bevy_gutzgutz::devtools::GutzDevtoolsPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window { title: "FUNKORO".into(), ..default() }),
        ..default()
    }))
    .add_plugins(game::FunkoroGamePlugin);

    #[cfg(feature = "dev")]
    app.add_plugins(GutzDevtoolsPlugin);

    app.run();
}
