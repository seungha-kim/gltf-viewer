use eframe::egui;

pub trait ViewContext<Model, Command> {
    fn model(&self) -> &Model;
    fn push_command(&mut self, command: Command);

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

pub trait ViewState<Model, Context: ViewContext<Model, Self::Command>> {
    type Command;

    fn interact(&mut self, ui: &mut egui::Ui, ctx: &Context);
    fn mutate(&mut self, ctx: &mut Context);

    fn update(&mut self, ui: &mut egui::Ui, ctx: &mut Context) {
        self.interact(ui, ctx);
        self.mutate(ctx);
    }
}
