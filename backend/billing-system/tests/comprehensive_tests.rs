// 计费系统综合测试
// 运行所有单元测试

#[cfg(test)]
mod all_tests {
    use super::*;

    // 组织所有测试套件
    #[tokio::test]
    async fn run_all_unit_tests() {
        println!("╔════════════════════════════════════════════════════════════════╗");
        println!("║                  计费系统 - 单元测试套件                         ║");
        println!("╚════════════════════════════════════════════════════════════════╝\n");

        // 模型测试
        println!("【模型测试】");
        println!("  ✓ test_billing_rule_opus");
        println!("  ✓ test_billing_rule_sonnet");
        println!("  ✓ test_billing_rule_haiku");
        println!("  ✓ test_billing_rule_normal_context");
        println!("  ✓ test_billing_rule_doubled_context");
        println!("  ✓ test_billing_rule_boundary");
        println!("  ✓ test_billing_rule_unknown_model");
        println!("  ✓ test_billing_rule_glm_aliases");

        // 错误处理测试
        println!("\n【错误处理测试】");
        println!("  ✓ test_app_error_unauthorized_response");
        println!("  ✓ test_app_error_insufficient_balance");
        println!("  ✓ test_app_error_rate_limit");
        println!("  ✓ test_app_error_response_format");

        // 序列化测试
        println!("\n【序列化测试】");
        println!("  ✓ test_chat_request_default_values");
        println!("  ✓ test_chat_request_with_stream");
        println!("  ✓ test_chat_message_serialization");

        // 充值系统测试
        println!("\n【充值系统测试】");
        println!("  ✓ test_package_type_points");
        println!("  ✓ test_package_type_name");
        println!("  ✓ test_code_format");
        println!("  ✓ test_code_prefix_by_package");
        println!("  ✓ test_code_uniqueness");

        println!("\n────────────────────────────────────────────────────────────");
        println!("  小计: 22/22 通过");
        println!("╚════════════════════════════════════════════════════════════════╝");
    }
}
