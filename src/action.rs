pub enum AppAction {
    SpawnWidget(WidgetType),
    ToggleSidebar,
    CloseActiveWidget,
    IncreaseScale,
    DecreaseScale,
}

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
}
