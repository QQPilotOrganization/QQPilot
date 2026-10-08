//! 内置模型：基于字符集合 Jaccard 相似度的极简问答（对应 C# 的 `TinyLangJaccard.cs`）。

use std::collections::{HashMap, HashSet};
use std::fs;

use crate::localization;

/// 从 `datasetTiny.json` 加载的"问题 -> 答案"表。
#[derive(Debug)]
pub struct TinyLangJaccard {
    qa_pairs: HashMap<String, String>,
    /// 保持 JSON 里的原始顺序，用于相似度打平时的取舍。
    questions: Vec<String>,
}

impl TinyLangJaccard {
    pub fn new(json_file_path: &str) -> Result<Self, String> {
        let translate = localization::load();
        let t = |key: &str| localization::get(&translate, key);

        let content = fs::read_to_string(json_file_path)
            .map_err(|e| format!("{} {json_file_path}: {e}", t("error.dataset.notfound")))?;

        let raw: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&content)
            .map_err(|e| format!("{} {json_file_path}: {e}", t("error.dataset.parsefailed")))?;

        let mut qa_pairs = HashMap::new();
        let mut questions = Vec::new();
        for (question, answer) in raw {
            let answer = answer.as_str().ok_or_else(|| {
                format!(
                    "{} {json_file_path} {question:?}",
                    t("error.dataset.badanswer")
                )
            })?;
            qa_pairs.insert(question.clone(), answer.to_string());
            questions.push(question);
        }

        Ok(Self {
            qa_pairs,
            questions,
        })
    }

    /// 对应 `TinyLangJaccardCS.Answer`：返回最相似问题对应的答案。
    pub fn answer(&self, question: &str) -> Result<String, String> {
        if self.questions.is_empty() {
            return Err(localization::get(
                &localization::load(),
                "error.dataset.empty",
            ));
        }

        let mut best_match = self.questions[0].clone();
        let mut max_similarity = -1.0f64;

        for candidate in &self.questions {
            let similarity = jaccard_similarity(question, candidate);
            if similarity > max_similarity {
                max_similarity = similarity;
                best_match = candidate.clone();
            }
        }

        let answer = self.qa_pairs.get(&best_match).cloned().unwrap_or_default();
        println!("{answer}");
        Ok(answer)
    }
}

/// 基于字符集合的 Jaccard 相似度。
fn jaccard_similarity(left: &str, right: &str) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let set1: HashSet<char> = left.chars().collect();
    let set2: HashSet<char> = right.chars().collect();

    let intersection = set1.intersection(&set2).count();
    let union = set1.union(&set2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    fn write_dataset(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        fs::write(&path, content).expect("写入临时数据集失败");
        path
    }

    fn dataset_str(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn jaccard_similarity_matches_the_csharp_rules() {
        // 字符集合完全相同 -> 1
        assert_eq!(jaccard_similarity("abc", "cba"), 1.0);
        // 两边都空 -> 1（C# 里也是这个特例）
        assert_eq!(jaccard_similarity("", ""), 1.0);
        // 只有一边空 -> 0
        assert_eq!(jaccard_similarity("abc", ""), 0.0);
        assert_eq!(jaccard_similarity("", "abc"), 0.0);
        // 完全不相交 -> 0
        assert_eq!(jaccard_similarity("abc", "xyz"), 0.0);
        // 交集 2 / 并集 3
        assert!((jaccard_similarity("你好", "你好啊") - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn answer_picks_the_most_similar_question() {
        let path = write_dataset(
            "qqpilot5-dataset-basic.json",
            r#"{"你好":"你好呀","再见":"拜拜","今天天气怎么样":"晴"}"#,
        );
        let tiny = TinyLangJaccard::new(&dataset_str(&path)).expect("加载数据集失败");

        assert_eq!(tiny.answer("你好啊").unwrap(), "你好呀");
        assert_eq!(tiny.answer("再见").unwrap(), "拜拜");
    }

    /// 打平时保留 JSON 里的先后顺序（C# 用严格大于比较，第一个最大值胜出）。
    #[test]
    fn ties_keep_the_first_question_in_file_order() {
        let path = write_dataset(
            "qqpilot5-dataset-tie.json",
            r#"{"ab":"first","ba":"second"}"#,
        );
        let tiny = TinyLangJaccard::new(&dataset_str(&path)).expect("加载数据集失败");

        assert_eq!(tiny.answer("ab").unwrap(), "first");
    }

    #[test]
    fn missing_dataset_reports_the_path() {
        let missing = std::env::temp_dir().join("qqpilot5-dataset-does-not-exist.json");
        let _ = fs::remove_file(&missing);

        let err = TinyLangJaccard::new(&dataset_str(&missing)).unwrap_err();
        assert!(
            err.contains("qqpilot5-dataset-does-not-exist.json"),
            "错误信息里应该带上路径，实际是: {err}"
        );
    }

    #[test]
    fn non_string_answer_is_rejected() {
        let path = write_dataset("qqpilot5-dataset-bad.json", r#"{"q": 42}"#);
        let err = TinyLangJaccard::new(&dataset_str(&path)).unwrap_err();
        assert!(err.contains("qqpilot5-dataset-bad.json"), "{err}");
    }

    #[test]
    fn empty_dataset_cannot_answer() {
        let path = write_dataset("qqpilot5-dataset-empty.json", "{}");
        let tiny = TinyLangJaccard::new(&dataset_str(&path)).expect("空数据集也应该能加载");

        assert!(tiny.answer("随便问问").is_err());
    }
}
