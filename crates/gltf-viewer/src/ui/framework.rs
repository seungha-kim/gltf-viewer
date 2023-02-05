use eframe::egui;

pub trait ViewContext {
    type Model;
    type Command;

    fn model(&self) -> &Self::Model;
    fn can_undo(&self) -> bool;
    fn can_redo(&self) -> bool;
    fn undo_requested(&self) -> bool;
    fn redo_requested(&self) -> bool;
    fn exit_requested(&self) -> bool;

    fn request_undo(&mut self);
    fn request_redo(&mut self);
    fn request_exit(&mut self);
    fn push_command(&mut self, command: Self::Command);
}

pub trait ViewState {
    type Context<'a>: ViewContext where Self: 'a;
    type Event;

    fn interact(&mut self, ui: &mut egui::Ui, ctx: &Self::Context<'_>, events: &mut Vec<Self::Event>);
    fn handle_view_event(&mut self, ctx: &mut Self::Context<'_>, event: Self::Event);

    fn update(&mut self, ui: &mut egui::Ui, ctx: &mut Self::Context<'_>) {
        let mut events: Vec<Self::Event> = Vec::new();
        self.interact(ui, ctx, &mut events);

        for event in events {
            self.handle_view_event(ctx, event);
        }
    }
}
