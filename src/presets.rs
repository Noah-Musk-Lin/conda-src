use crate::models::MirrorProfile;
use std::collections::BTreeMap;

/// 返回内置的可用 Conda 镜像源预设。
/// 注意：不包含已停止服务的 TUNA Anaconda 镜像。
pub fn builtin_mirrors() -> Vec<MirrorProfile> {
    let mut pku_custom = BTreeMap::new();
    pku_custom.insert(
        "conda-forge".to_string(),
        "https://mirrors.pku.edu.cn/anaconda/cloud".to_string(),
    );
    pku_custom.insert(
        "pytorch".to_string(),
        "https://mirrors.pku.edu.cn/anaconda/cloud".to_string(),
    );
    pku_custom.insert(
        "bioconda".to_string(),
        "https://mirrors.pku.edu.cn/anaconda/cloud".to_string(),
    );

    let mut official_custom = BTreeMap::new();
    official_custom.insert(
        "conda-forge".to_string(),
        "https://conda.anaconda.org".to_string(),
    );
    official_custom.insert(
        "pytorch".to_string(),
        "https://conda.anaconda.org".to_string(),
    );
    official_custom.insert(
        "bioconda".to_string(),
        "https://conda.anaconda.org".to_string(),
    );

    let mut ustc_custom = BTreeMap::new();
    ustc_custom.insert(
        "conda-forge".to_string(),
        "https://mirrors.ustc.edu.cn/anaconda/cloud".to_string(),
    );
    ustc_custom.insert(
        "pytorch".to_string(),
        "https://mirrors.ustc.edu.cn/anaconda/cloud".to_string(),
    );
    ustc_custom.insert(
        "bioconda".to_string(),
        "https://mirrors.ustc.edu.cn/anaconda/cloud".to_string(),
    );

    vec![
        MirrorProfile {
            id: "pku".to_string(),
            name: "北京大学镜像站 (PKU)".to_string(),
            description: "国内高速稳定，覆盖 main / r / msys2 及 conda-forge / pytorch / bioconda".to_string(),
            is_builtin: true,
            default_channels: vec![
                "https://mirrors.pku.edu.cn/anaconda/pkgs/main".to_string(),
                "https://mirrors.pku.edu.cn/anaconda/pkgs/r".to_string(),
                "https://mirrors.pku.edu.cn/anaconda/pkgs/msys2".to_string(),
            ],
            custom_channels: pku_custom,
            probe_url: "https://mirrors.pku.edu.cn/anaconda/cloud/conda-forge/noarch/repodata.json"
                .to_string(),
        },
        MirrorProfile {
            id: "official".to_string(),
            name: "Conda 官方源 (Official)".to_string(),
            description: "Anaconda 与 conda-forge 上游官方源，包版本最新、同步无延迟".to_string(),
            is_builtin: true,
            default_channels: vec![
                "https://repo.anaconda.com/pkgs/main".to_string(),
                "https://repo.anaconda.com/pkgs/r".to_string(),
                "https://repo.anaconda.com/pkgs/msys2".to_string(),
            ],
            custom_channels: official_custom,
            probe_url: "https://conda.anaconda.org/conda-forge/noarch/repodata.json".to_string(),
        },
        MirrorProfile {
            id: "ustc".to_string(),
            name: "中科大镜像站 (USTC)".to_string(),
            description: "中国科学技术大学开源镜像站，适合作为国内备用镜像源".to_string(),
            is_builtin: true,
            default_channels: vec![
                "https://mirrors.ustc.edu.cn/anaconda/pkgs/main".to_string(),
                "https://mirrors.ustc.edu.cn/anaconda/pkgs/r".to_string(),
                "https://mirrors.ustc.edu.cn/anaconda/pkgs/msys2".to_string(),
            ],
            custom_channels: ustc_custom,
            probe_url: "https://mirrors.ustc.edu.cn/anaconda/cloud/conda-forge/noarch/repodata.json"
                .to_string(),
        },
    ]
}
