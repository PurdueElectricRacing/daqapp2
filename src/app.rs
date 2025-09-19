use eframe::egui;
use crate::widgets::Widget;
use crate::can_viewer::CanViewer;
use crate::bootloader::Bootloader;
use crate::live_plot::LivePlot;

pub struct DAQApp {
    pub is_sidebar_open: bool,
    pub tile_tree: egui_tiles::Tree<Widget>,
    pub next_can_viewer_num: usize,
    pub next_bootloader_num: usize,
    pub next_live_plot_num: usize,
}

impl Default for DAQApp {
    fn default() -> Self {
        Self {
            is_sidebar_open: true,
            tile_tree: egui_tiles::Tree::empty("workspace_tree"),
            next_can_viewer_num: 1,
            next_bootloader_num: 1,
            next_live_plot_num: 1,
        }
    }
}

impl DAQApp {
    /// Add a widget to the tile tree, handling root management and tab containers
    fn add_widget_to_tree(&mut self, widget: Widget) {
        let new_tile_id = self.tile_tree.tiles.insert_pane(widget);
        
        if let Some(root_id) = self.tile_tree.root {
            // If we have a root, create a new tab container or add to existing one
            if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) = 
                self.tile_tree.tiles.get_mut(root_id) 
            {
                // Root is already a tab container, add to it
                tabs.add_child(new_tile_id);
                tabs.set_active(new_tile_id);
            } else {
                // Root is not a tab container, create one
                let tab_container = self.tile_tree.tiles.insert_tab_tile(vec![root_id, new_tile_id]);
                self.tile_tree.root = Some(tab_container);
            }
        } else {
            // No root yet, this becomes the root
            self.tile_tree.root = Some(new_tile_id);
        }
    }
    
    pub fn spawn_can_viewer(&mut self) {
        let widget = Widget::CanViewer(CanViewer::new(self.next_can_viewer_num));
        self.next_can_viewer_num += 1;
        self.add_widget_to_tree(widget);
    }
    
    pub fn spawn_bootloader(&mut self) {
        let widget = Widget::Bootloader(Bootloader::new(self.next_bootloader_num));
        self.next_bootloader_num += 1;
        self.add_widget_to_tree(widget);
    }
    
    pub fn spawn_live_plot(&mut self) {
        let widget = Widget::LivePlot(LivePlot::new(self.next_live_plot_num));
        self.next_live_plot_num += 1;
        self.add_widget_to_tree(widget);
    }
}

impl eframe::App for DAQApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // a tiny toolbar button to toggle the sidebar
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            if ui.button(if self.is_sidebar_open { "Hide side bar" } else { "Show sidebar" }).clicked() {
                self.is_sidebar_open = !self.is_sidebar_open;
            }
        });

        // sidebar
        crate::sidebar::show(self, ctx);

        // workspace (central panel)
        crate::workspace::show(self, ctx);
    }
}