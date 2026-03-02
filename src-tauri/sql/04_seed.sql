-- 种子数据
-- 默认 AI 功能配置

INSERT OR IGNORE INTO ai_features (feature_name, enabled, config_json)
VALUES ('chat', 1, '{"max_history":100,"auto_save":true}');

INSERT OR IGNORE INTO ai_features (feature_name, enabled, config_json)
VALUES ('explanation', 1, '{"auto_explain":false}');

INSERT OR IGNORE INTO ai_features (feature_name, enabled, config_json)
VALUES ('command_search', 1, '{"max_results":50}');
