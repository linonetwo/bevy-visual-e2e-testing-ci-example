use bevy::prelude::*;
use log::{info, warn};
use std::fs;
use std::path::Path;

/// 字体配置
#[derive(Debug, Clone, Default)]
pub struct FontConfig {
    /// 字体路径或名称
    pub font_path: Option<String>,
}

/// 获取系统默认字体路径列表（按优先级排序）
fn get_default_font_paths() -> Vec<&'static str> {
    if cfg!(target_os = "windows") {
        vec![
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            // 优先使用中文字体
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            // 备用英文字体
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ]
    } else {
        // macOS
        vec![
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ]
    }
}

/// 尝试加载字体文件
fn try_load_font(font_path: &str) -> Option<Font> {
    match fs::read(font_path) {
        Ok(font_data) => match Font::try_from_bytes(font_data) {
            Ok(font) => {
                info!("成功加载字体: {}", font_path);
                Some(font)
            }
            Err(e) => {
                warn!("字体解析失败 {}: {:?}", font_path, e);
                None
            }
        },
        Err(e) => {
            warn!("无法读取字体文件 {}: {}", font_path, e);
            None
        }
    }
}

/// 加载字体并设置为默认字体
///
/// # Arguments
/// * `world` - Bevy 世界
/// * `config` - 字体配置
///
/// # Returns
/// 是否成功加载自定义字体
pub fn load_and_set_default_font(world: &mut World, config: &FontConfig) -> bool {
    // 如果用户指定了字体路径，优先使用
    if let Some(custom_path) = &config.font_path {
        if let Some(font) = try_load_font(custom_path) {
            set_default_font(world, font, custom_path);
            return true;
        }
        warn!("用户指定字体加载失败: {}", custom_path);
    }

    // 尝试系统默认字体（按优先级）
    for font_path in get_default_font_paths() {
        if let Some(font) = try_load_font(font_path) {
            set_default_font(world, font, font_path);
            return true;
        }
    }

    // 所有字体加载都失败，使用 Bevy 默认字体
    warn!("使用 Bevy 内置默认字体");
    false
}

// 设置默认字体的辅助函数
fn set_default_font(world: &mut World, font: Font, path: &str) {
    let default_font_handle = TextFont::default().font.clone();
    let _ = world
        .resource_mut::<Assets<Font>>()
        .insert(&default_font_handle, font);
    info!("已设置默认字体: {}", path);
}

/// 加载字体资源（不设置为默认）
///
/// # Arguments
/// * `asset_server` - Bevy 资源服务器
/// * `font_path` - 字体文件路径
///
/// # Returns
/// 字体句柄（如果加载成功）
#[allow(dead_code)]
pub fn load_font_asset(asset_server: &AssetServer, font_path: String) -> Option<Handle<Font>> {
    if Path::new(&font_path).exists() {
        info!("加载字体资源: {}", font_path);
        Some(asset_server.load(font_path))
    } else {
        warn!("字体文件不存在: {}", font_path);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_font_paths() {
        let paths = get_default_font_paths();
        assert!(!paths.is_empty());
        assert!(!paths[0].is_empty());
    }

    #[test]
    fn test_font_config_with_custom_path() {
        let config = FontConfig {
            font_path: Some("/custom/font.ttf".to_string()),
        };
        assert_eq!(config.font_path.as_deref(), Some("/custom/font.ttf"));
    }
}
