use crate::activity::DynActivity;

pub trait IntoActivity {
    fn into_activity(self) -> Box<dyn DynActivity>;
}
