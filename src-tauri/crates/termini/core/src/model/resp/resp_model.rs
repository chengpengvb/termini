use serde::Serialize;
use summer_web::axum::Json;
use summer::tracing::error;
use summer_web::axum::http::StatusCode;
use summer_web::axum::response::{IntoResponse, Response};

pub const ERROR_CODE: i32 = ResultCode::Error as i32;
pub const SUCCESS_CODE: i32 = ResultCode::Success as i32;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultCode {
    Error = 500,
    Success = 200,
    Unauthorized = 401,
    AiModelUnavailable = 40001,
    Cancelled = 40002,
    ImagesUnsupported = 40004,
    AiModelNotFound = 40005,
    NoAccount = 40006,
    NoBaseUrl = 40007,
    BillingRate = 40008,
    Quota = 41009,
    Subscription = 41010,
    UserDisabled = 41011,
    DeviceLimitExceeded = 41012,
    ProRequired = 41013,
    SuggestLimit = 41014,
    ChatNotFound = 40011,
    ChatTitleEmpty = 40012,
    ChatGenerating = 40013,
    ChatItemNotFound = 40014,
    ChatTypeUnknown = 40015,
    ProjectNotFound = 40016,
    ProjectNameEmpty = 40017,
    JobDeviceNotFound = 40018,
    TokenEmpty = 40019,
    NotLoggedIn = 40020,
    UrlEmpty = 40021,
    InvalidUrl = 40022,
    UnsupportedUrlScheme = 40023,
    DefaultConfigNotFound = 40024,
    LoginUrlEmpty = 40025,
    SuggestProvideEmpty = 40026,
    SuggestCountRange = 40027,
    GroupNotFound = 40028,
    GroupNameEmpty = 40029,
    GroupNameExists = 40030,
    CloudSyncBusy = 40031,
    CloudSyncOpInvalid = 40032,
    CloudSyncChoiceInvalid = 40033,
    PasswordTooShort = 40034,
    PasswordTooLong = 40035,
    PasswordMismatch = 40036,
    CloudSyncNotFound = 40037,
    McpServerNotFound = 40038,
    NameEmpty = 40039,
    McpStdioCommandRequired = 40040,
    McpHttpUrlRequired = 40041,
    McpTransportUnsupported = 40042,
    TimeoutSecsInvalid = 40043,
    SkillNameEmpty = 40044,
    SkillContentEmpty = 40045,
    SkillDescriptionTooLong = 40046,
    IndicatorDeviceRequired = 40047,
    IndicatorNameEmpty = 40048,
    IndicatorTransformInvalid = 40049,
    IndicatorDeleted = 40050,
    JobDeviceIdRequired = 40051,
    JobIndicatorIdRequired = 40052,
    DiscoverInProgress = 40053,
    DiscoverModelNotSet = 40054,
    DiscoverEnded = 40055,
    DiscoverNotReady = 40056,
    DiscoverQueuedCancelled = 40057,
    DiscoverCollectConfigMissing = 40058,
    DeviceDeleted = 40059,
    SkillTaskIdEmpty = 40060,
    SkillTaskRunning = 40061,
    SkillTaskNotFound = 40062,
    SkillExtractInputRequired = 40063,
    ProviderNotFound = 40064,
    ExecTimeoutInvalid = 40065,
    SkillExtractEmpty = 40066,
    CloudSyncNoData = 40067,
    DeviceNotFound = 40068,
    JsonArrayRequired = 40069,
    JsonObjectRequired = 40070,
    ClipboardWriteFailed = 40071,
    HapticUnsupported = 40072,
    WindowOpUnsupported = 40073,
    WindowOpFailed = 40074,
    MainWindowNotFound = 40075,
    DragUnsupported = 40076,
    SessionIdEmpty = 40077,
    DeviceIdEmpty = 40078,
    DeviceIdLocal = 40079,
    SshShellNotOpened = 40080,
    SshConfigNotFound = 40081,
    ConfigIdMismatch = 40082,
    SkillExtractParseFailed = 40083,
    OpenUrlFailed = 40084,
    ModelMaxTokenMissing = 40085,
    ClipboardReadFailed = 40086,
    ChatImageNotFound = 40087,
    ChatImageSaveFailed = 40088,
    StreamIdleTimeout = 40089,
    LocalPathNotFound = 40090,
    OpenLocalPathFailed = 40091,
    GroupDefaultProtected = 40092,
    Duplicate = 40093,
}

#[derive(Debug, Clone)]
pub struct BizError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ResultModel<T: Serialize> {
    pub code: i32,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl ResultCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

impl BizError {
    pub fn new(code: ResultCode) -> Self {
        Self { code: code.as_i32(), message: String::new() }
    }

    pub fn duplicate(existing_id: String) -> Self {
        Self { code: ResultCode::Duplicate.as_i32(), message: existing_id }
    }
}

impl From<ResultCode> for anyhow::Error {
    fn from(code: ResultCode) -> Self {
        anyhow::Error::from(BizError::new(code))
    }
}

impl std::fmt::Display for BizError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "code={} message={}", self.code, self.message)
    }
}

impl std::error::Error for BizError {}

impl<T: Serialize> ResultModel<T> {
    pub fn ok(data: T) -> Self {
        Self { code: SUCCESS_CODE, message: Some("OK".to_string()), data: Some(data) }
    }

    pub fn fail(message: impl Into<String>) -> Self {
        Self { code: ERROR_CODE, message: Some(message.into()), data: None }
    }

    pub fn fail_code(code: ResultCode) -> Self {
        Self { code: code.as_i32(), message: None, data: None }
    }

    pub fn from_result(r: anyhow::Result<T>) -> Self {
        match r {
            Ok(data) => Self::ok(data),
            Err(e) => {
                if let Some(biz) = e.downcast_ref::<BizError>() {
                    error!("[result] biz error code={} message={}", biz.code, biz.message);
                    let message = if biz.message.is_empty() { None } else { Some(biz.message.clone()) };
                    return Self { code: biz.code, message, data: None };
                }
                error!("[result] error={:#}", e);
                Self::fail(format!("{e:?}"))
            }
        }
    }
}

impl<T: Serialize> IntoResponse for ResultModel<T> {
    fn into_response(self) -> Response {
        let status = if self.code == ResultCode::Unauthorized as i32 { StatusCode::UNAUTHORIZED } else { StatusCode::OK };
        (status, Json(self)).into_response()
    }
}
