//! 应用配置模块
//!
//! 通过 `build.rs` 在编译时将环境变量内嵌到二进制文件中，
//! 运行时也可通过同名环境变量覆盖。

/// 应用更新配置
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// 版本检查接口地址
    pub update_url: String,
    /// 更新下载页地址（可选）
    pub download_url: Option<String>,
}

impl UpdateConfig {
    /// 获取更新配置
    ///
    /// 优先级：运行时环境变量 > 编译时环境变量
    pub fn get() -> Self {
        Self {
            update_url: Self::get_update_url(),
            download_url: Self::get_download_url(),
        }
    }

    /// 获取更新接口地址
    fn get_update_url() -> String {
        if let Ok(url) = std::env::var("PIG_UPDATE_URL") {
            if !url.trim().is_empty() {
                return url;
            }
        }

        option_env!("PIG_UPDATE_URL")
            .unwrap_or_default()
            .to_string()
    }

    /// 获取下载页地址
    fn get_download_url() -> Option<String> {
        if let Ok(url) = std::env::var("PIG_UPDATE_DOWNLOAD_URL") {
            if !url.trim().is_empty() {
                return Some(url);
            }
        }

        option_env!("PIG_UPDATE_DOWNLOAD_URL").and_then(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        })
    }

    /// 检查配置是否有效
    pub fn is_valid(&self) -> bool {
        !self.update_url.trim().is_empty()
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self::get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_config_get() {
        let config = UpdateConfig::get();
        let _ = config.is_valid();
    }
}
