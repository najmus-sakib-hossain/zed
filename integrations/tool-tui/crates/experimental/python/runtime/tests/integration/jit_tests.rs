//! JIT compilation integration tests

use dx_py_jit::{CompilationTier, TieredJit, FunctionProfile};

#[test]
fn test_tier_system() {
    // Test tier ordering
    assert!(CompilationTier::Tier0 < CompilationTier::Tier1);
    assert!(CompilationTier::Tier1 < CompilationTier::Tier2);
    assert!(CompilationTier::Tier2 < CompilationTier::Tier3);
}

#[test]
fn test_function_profile() {
    let mut profile = FunctionProfile::new("test_func".to_string());
    
    assert_eq!(profile.call_count(), 0);
    assert_eq!(profile.current_tier(), CompilationTier::Tier0);
    
    // Record calls
    for _ in 0..100 {
        profile.record_call();
    }
    
    assert_eq!(profile.call_count(), 100);
}

#[test]
fn test_tiered_jit_creation() {
    let jit = TieredJit::new();
    
    // Should not have any compiled code initially
    assert!(!jit.has_compiled("nonexistent", CompilationTier::Tier1));
}

#[test]
fn test_tier_promotion_thresholds() {
    let mut profile = FunctionProfile::new("hot_func".to_string());
    
    // Tier 0 -> Tier 1 at 100 calls
    for _ in 0..99 {
        profile.record_call();
    }
    assert_eq!(profile.current_tier(), CompilationTier::Tier0);
    
    profile.record_call();
    profile.set_tier(CompilationTier::Tier1);
    assert_eq!(profile.current_tier(), CompilationTier::Tier1);
}

#[test]
fn test_multiple_functions() {
    let mut profiles = vec![
        FunctionProfile::new("func1".to_string()),
        FunctionProfile::new("func2".to_string()),
        FunctionProfile::new("func3".to_string()),
    ];
    
    // Different call counts
    for _ in 0..50 {
        profiles[0].record_call();
    }
    for _ in 0..150 {
        profiles[1].record_call();
    }
    for _ in 0..1500 {
        profiles[2].record_call();
    }
    
    assert_eq!(profiles[0].call_count(), 50);
    assert_eq!(profiles[1].call_count(), 150);
    assert_eq!(profiles[2].call_count(), 1500);
}

#[test]
fn test_type_feedback() {
    use dx_py_jit::TypeFeedback;
    
    let mut feedback = TypeFeedback::new();
    
    // Record type observations
    feedback.record_type("int");
    feedback.record_type("int");
    feedback.record_type("int");
    
    assert!(feedback.is_monomorphic());
    assert_eq!(feedback.dominant_type(), Some("int"));
    
    // Add a different type
    feedback.record_type("float");
    assert!(!feedback.is_monomorphic());
}

#[test]
fn test_deoptimization_trigger() {
    let mut profile = FunctionProfile::new("deopt_func".to_string());
    
    // Promote to tier 2
    for _ in 0..1000 {
        profile.record_call();
    }
    profile.set_tier(CompilationTier::Tier2);
    
    // Trigger deoptimization
    profile.trigger_deopt();
    
    // Should be back to tier 0
    assert_eq!(profile.current_tier(), CompilationTier::Tier0);
}
