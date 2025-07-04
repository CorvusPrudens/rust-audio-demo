use bevy::prelude::*;

mod dialog;
mod sequence;

pub fn textbox_plugin(app: &mut App) {
    app.add_plugins((
        bevy_sequence::SequencePlugin,
        bevy_pretty_text::PrettyTextPlugin,
        sequence::sequence_plugin,
        dialog::dialog_plugin,
    ));
}
