pub enum AppAction {
    SpawnWidget(WidgetType),
    ToggleSidebar,
    ToggleCommandPalette,
    CloseActiveWidget,
    IncreaseScale,
    DecreaseScale,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WidgetType {
    ViewerTable,
    ViewerList,
    Bootloader,
    Scope {
        msg_id: u32,
        msg_name: String,
        signal_name: String,
    },
    LogParser,
    SendUi,
    BusLoad,
}

impl AppAction {
    pub fn cmd_palette_list() -> Vec<(&'static str, WidgetType)> {
        vec![
            ("Spawn CAN Table", WidgetType::ViewerTable),
            ("Spawn CAN List", WidgetType::ViewerList),
            ("Spawn Bootloader", WidgetType::Bootloader),
            ("Spawn Log Parser", WidgetType::LogParser),
            ("Spawn Send UI", WidgetType::SendUi),
            ("Spawn Bus Load", WidgetType::BusLoad),
        ]
    }
}
