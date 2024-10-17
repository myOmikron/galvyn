use axum::Router;

pub trait Module {
    fn name(&self) -> &str;

    fn init_stage(&self) {}

    fn router_stage(&self, root: Router) -> Router {
        root
    }
}
