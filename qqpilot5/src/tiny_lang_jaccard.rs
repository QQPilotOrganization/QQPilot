//! 内置模型：基于字符集合 Jaccard 相似度的极简问答（对应 C# 的 `TinyLangJaccard.cs`）。

use std::collections::{HashMap, HashSet};
use std::fs;

/// 从 `datasetTiny.json` 加载的"问题 -> 答案"表。
pub struct TinyLangJaccard {
    qa_pairs: HashMap<String, String>,
    /// 保持 JSON 里的原始顺序，用于相似度打平时的取舍。
    questions: Vec<String>,
}

impl TinyLangJaccard {
    pub fn new(json_file_path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(json_file_path)
            .map_err(|e| format!("找不到数据集文件 {json_file_path}: {e}"))?;

        let raw: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&content)
            .map_err(|e| format!("解析 {json_file_path} 失败: {e}"))?;

        let mut qa_pairs = HashMap::new();
        let mut questions = Vec::new();
        for (question, answer) in raw {
            let answer = answer
                .as_str()
                .ok_or_else(|| format!("{json_file_path} 中 {question:?} 对应的答案不是字符串"))?;
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
            return Err("数据集中没有问题".to_string());
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
