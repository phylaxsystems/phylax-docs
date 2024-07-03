/// Macro for generating a mock activity implementation.
///
/// This macro defines an implementation of the Activity trait for a given struct type, along with
/// associated types and async logic.
///
/// # Parameters
/// - `$struct_type`: The name of the struct type for which the activity is being mocked.
/// - `$input_type`: The type of input expected by the activity.
/// - `$output_type`: The type of output produced by the activity.
/// - `$sel`: The selector reference for the activity.
/// - `$inp`: The input parameter for the activity.
/// - `$ctx`: The activity context parameter.
/// - `$async_logic`: The async block defining the processing logic of the activity.
///
/// # Example
/// ```ignore
/// # Since we are not importing the interfaces crate here, we're not compiling this rustdoc until the test-utils crate gains it as a dependency.
/// use phylax_test_utils::mock_activity;
///
/// pub struct MockActivity;
///
/// mock_activity!(MockActivity, String, u32, self input context {
///     Some(async {
///         // Processing logic here
///         Ok(42)
///     })
/// });
/// ```
#[macro_export]
macro_rules! mock_activity {
    ($struct_type:ident, $input_type:ty, $output_type:ty, $sel:ident $inp:ident $ctx:ident $async_logic:block) => {
        impl Activity for $struct_type {
            const FLAVOR_NAME: &'static str = stringify!($struct_type);
            type Input = $input_type;
            type Output = $output_type;

            async fn process_message(
                &mut $sel,
                $inp: Option<(Self::Input, MessageDetails)>,
                $ctx: ActivityContext,
            ) -> Result<Option<Self::Output>, PhylaxError> $async_logic
        }
    };
}
