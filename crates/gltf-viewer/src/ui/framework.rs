use eframe::egui;

pub trait ViewContext<M, CMD> {
    fn model(&self) -> &M;
    fn push_command(&mut self, command: CMD);

    fn exit_requested(&self) -> bool;
    fn request_exit(&mut self);
}

pub trait UndoableViewContext {
    fn can_undo(&self) -> bool;
    fn can_redo(&self) -> bool;
    fn undo_requested(&self) -> bool;
    fn redo_requested(&self) -> bool;

    fn request_undo(&mut self);
    fn request_redo(&mut self);
}

pub trait ViewState<M, CTX: ViewContext<M, Self::Command>> {
    type Command;
    type Event;

    fn interact(&mut self, ui: &mut egui::Ui, ctx: &CTX, events: &mut Vec<Self::Event>);
    fn handle_view_event(&mut self, ctx: &mut CTX, event: Self::Event);

    fn update(&mut self, ui: &mut egui::Ui, ctx: &mut CTX) {
        let mut events: Vec<Self::Event> = Vec::new();
        self.interact(ui, ctx, &mut events);

        for event in events {
            self.handle_view_event(ctx, event);
        }
    }
}
