use bevy::prelude::*;
use bevy_egui::{EguiContext, PrimaryEguiContext, egui};

/// World Inspectorウィンドウの表示/非表示。`draw_overlay`のチェックボックスから
/// トグルする。
#[derive(Resource, Default)]
pub struct GutzInspectorVisible(pub bool);

/// Entity/Resourceを丸ごと閲覧できる生インスペクタ。`bevy-inspector-egui`の
/// `ui_for_world`をそのまま呼ぶだけの薄い配線——ゲーム固有のフィルタリングや
/// 見た目はここに持ち込まない（devtools全体の方針と同じ）。
///
/// `&mut World`を直接取る排他システムにしているのは、`ui_for_world`自体が
/// Entity・Resource・Assetを横断して読み書きするため、通常の`Query`/`Res`の
/// 組み合わせでは表現できないため（bevy-inspector-egui公式のworld_inspector
/// exampleと同じ形）。`EguiContext`は`Clone`なComponentなので、先に複製して
/// から`world`を自由に借用し直す。
pub(crate) fn draw_world_inspector(world: &mut World) {
    let Ok(egui_context) =
        world.query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>().single_mut(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    egui::Window::new("gutzgutz devtools: World Inspector").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            bevy_inspector_egui::bevy_inspector::ui_for_world(world, ui);
        });
    });
}
