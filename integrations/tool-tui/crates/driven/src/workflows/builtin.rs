//! Built-in Workflow Definitions
//!
//! Provides 30+ guided workflows matching BMAD-METHOD capabilities.

use super::{Workflow, WorkflowPhase, WorkflowStep};

/// Get all built-in workflows
pub fn all_workflows() -> Vec<Workflow> {
    let mut workflows = Vec::new();

    // Analysis Phase (4)
    workflows.extend(analysis_workflows());

    // Planning Phase (4)
    workflows.extend(planning_workflows());

    // Solutioning Phase (4)
    workflows.extend(solutioning_workflows());

    // Implementation Phase (5)
    workflows.extend(implementation_workflows());

    // Quick Flow (3)
    workflows.extend(quick_flow_workflows());

    // Testing (3)
    workflows.extend(testing_workflows());

    // Documentation (2)
    workflows.extend(documentation_workflows());

    // DevOps (2)
    workflows.extend(devops_workflows());

    // Additional workflows (7+)
    workflows.extend(additional_workflows());

    workflows
}

/// Analysis phase workflows
pub fn analysis_workflows() -> Vec<Workflow> {
    vec![
        brainstorming(),
        research(),
        product_brief(),
        competitive_analysis(),
    ]
}

/// Planning phase workflows
pub fn planning_workflows() -> Vec<Workflow> {
    vec![prd(), ux_design(), tech_spec(), api_design()]
}

/// Solutioning phase workflows
pub fn solutioning_workflows() -> Vec<Workflow> {
    vec![
        architecture(),
        epics_and_stories(),
        implementation_readiness(),
        data_model(),
    ]
}

/// Implementation phase workflows
pub fn implementation_workflows() -> Vec<Workflow> {
    vec![
        sprint_planning(),
        dev_story(),
        code_review(),
        retrospective(),
        sprint_status(),
    ]
}

/// Quick Flow workflows
pub fn quick_flow_workflows() -> Vec<Workflow> {
    vec![quick_bug_fix(), quick_feature(), quick_refactor()]
}

/// Testing workflows
pub fn testing_workflows() -> Vec<Workflow> {
    vec![test_design(), test_automation(), test_review()]
}

/// Documentation workflows
pub fn documentation_workflows() -> Vec<Workflow> {
    vec![document_project(), api_documentation()]
}

/// DevOps workflows
pub fn devops_workflows() -> Vec<Workflow> {
    vec![ci_setup(), deployment()]
}

/// Additional workflows
pub fn additional_workflows() -> Vec<Workflow> {
    vec![
        security_review(),
        performance_review(),
        onboarding(),
        tech_debt(),
        migration(),
        incident_response(),
        release_planning(),
    ]
}

// ============ Analysis Phase ============

pub fn brainstorming() -> Workflow {
    Workflow::new(
        "brainstorming",
        "Brainstorming",
        WorkflowPhase::Analysis,
        "Generate and explore ideas for a new feature or product",
    )
    .with_step(
        WorkflowStep::new(
            "define-problem",
            "Define the Problem",
            "Clearly articulate the problem to solve",
            "analyst",
        )
        .with_actions(vec![
            "Identify the core problem".to_string(),
            "Define success criteria".to_string(),
            "List constraints and assumptions".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new(
            "generate-ideas",
            "Generate Ideas",
            "Brainstorm potential solutions",
            "analyst",
        )
        .with_actions(vec![
            "List all possible solutions".to_string(),
            "Encourage wild ideas".to_string(),
            "Build on others' ideas".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new(
            "evaluate-ideas",
            "Evaluate Ideas",
            "Assess and prioritize ideas",
            "analyst",
        )
        .with_actions(vec![
            "Score ideas against criteria".to_string(),
            "Identify pros and cons".to_string(),
            "Select top candidates".to_string(),
        ])
        .as_checkpoint(),
    )
    .as_builtin()
}

pub fn research() -> Workflow {
    Workflow::new(
        "research",
        "Research",
        WorkflowPhase::Analysis,
        "Conduct research to inform product decisions",
    )
    .with_step(
        WorkflowStep::new(
            "define-questions",
            "Define Research Questions",
            "Identify what we need to learn",
            "analyst",
        )
        .with_actions(vec![
            "List key questions".to_string(),
            "Prioritize by impact".to_string(),
            "Define research methods".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("gather-data", "Gather Data", "Collect relevant information", "analyst")
            .with_actions(vec![
                "Review existing documentation".to_string(),
                "Analyze competitors".to_string(),
                "Interview stakeholders".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new(
            "synthesize",
            "Synthesize Findings",
            "Analyze and summarize research",
            "analyst",
        )
        .with_actions(vec![
            "Identify patterns and themes".to_string(),
            "Draw conclusions".to_string(),
            "Make recommendations".to_string(),
        ])
        .as_checkpoint(),
    )
    .as_builtin()
}

pub fn product_brief() -> Workflow {
    Workflow::new(
        "product-brief",
        "Product Brief",
        WorkflowPhase::Analysis,
        "Create a concise product brief",
    )
    .with_step(
        WorkflowStep::new("vision", "Define Vision", "Articulate the product vision", "pm")
            .with_actions(vec![
                "Write vision statement".to_string(),
                "Define target users".to_string(),
                "Identify key benefits".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("scope", "Define Scope", "Outline what's in and out of scope", "pm")
            .with_actions(vec![
                "List core features".to_string(),
                "Define MVP boundaries".to_string(),
                "Identify future phases".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new(
            "success-metrics",
            "Define Success Metrics",
            "How we'll measure success",
            "pm",
        )
        .with_actions(vec![
            "Define KPIs".to_string(),
            "Set targets".to_string(),
            "Plan measurement approach".to_string(),
        ])
        .as_checkpoint(),
    )
    .as_builtin()
}

pub fn competitive_analysis() -> Workflow {
    Workflow::new(
        "competitive-analysis",
        "Competitive Analysis",
        WorkflowPhase::Analysis,
        "Analyze competitors and market positioning",
    )
    .with_step(
        WorkflowStep::new(
            "identify-competitors",
            "Identify Competitors",
            "List direct and indirect competitors",
            "analyst",
        )
        .with_actions(vec![
            "Research market landscape".to_string(),
            "Categorize competitors".to_string(),
            "Prioritize for analysis".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new(
            "analyze-features",
            "Analyze Features",
            "Compare feature sets",
            "analyst",
        )
        .with_actions(vec![
            "Create feature matrix".to_string(),
            "Identify gaps and opportunities".to_string(),
            "Note unique differentiators".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new(
            "positioning",
            "Define Positioning",
            "Determine market positioning",
            "pm",
        )
        .with_actions(vec![
            "Identify unique value proposition".to_string(),
            "Define competitive advantages".to_string(),
            "Create positioning statement".to_string(),
        ])
        .as_checkpoint(),
    )
    .as_builtin()
}

// ============ Planning Phase ============

pub fn prd() -> Workflow {
    Workflow::new(
        "prd",
        "Product Requirements Document",
        WorkflowPhase::Planning,
        "Create a comprehensive PRD",
    )
    .with_step(
        WorkflowStep::new("overview", "Write Overview", "Document product overview", "pm")
            .with_actions(vec![
                "Write executive summary".to_string(),
                "Define problem statement".to_string(),
                "List objectives".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new(
            "requirements",
            "Define Requirements",
            "Document functional requirements",
            "pm",
        )
        .with_actions(vec![
            "Write user stories".to_string(),
            "Define acceptance criteria".to_string(),
            "Prioritize requirements".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new(
            "non-functional",
            "Non-Functional Requirements",
            "Document NFRs",
            "architect",
        )
        .with_actions(vec![
            "Define performance requirements".to_string(),
            "Define security requirements".to_string(),
            "Define scalability requirements".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("review", "Review PRD", "Get stakeholder approval", "pm")
            .with_actions(vec![
                "Share with stakeholders".to_string(),
                "Collect feedback".to_string(),
                "Finalize document".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn ux_design() -> Workflow {
    Workflow::new(
        "ux-design",
        "UX Design",
        WorkflowPhase::Planning,
        "Design user experience and interface",
    )
    .with_step(
        WorkflowStep::new("user-research", "User Research", "Understand user needs", "ux-designer")
            .with_actions(vec![
                "Create user personas".to_string(),
                "Map user journeys".to_string(),
                "Identify pain points".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new(
            "wireframes",
            "Create Wireframes",
            "Design low-fidelity wireframes",
            "ux-designer",
        )
        .with_actions(vec![
            "Sketch key screens".to_string(),
            "Define information architecture".to_string(),
            "Plan navigation flow".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new(
            "prototype",
            "Build Prototype",
            "Create interactive prototype",
            "ux-designer",
        )
        .with_actions(vec![
            "Build clickable prototype".to_string(),
            "Add interactions".to_string(),
            "Prepare for testing".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("validate", "Validate Design", "Test with users", "ux-designer")
            .with_actions(vec![
                "Conduct usability testing".to_string(),
                "Gather feedback".to_string(),
                "Iterate on design".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn tech_spec() -> Workflow {
    Workflow::new(
        "tech-spec",
        "Technical Specification",
        WorkflowPhase::Planning,
        "Create detailed technical specification",
    )
    .with_step(
        WorkflowStep::new(
            "overview",
            "Technical Overview",
            "Document technical approach",
            "architect",
        )
        .with_actions(vec![
            "Describe solution approach".to_string(),
            "List technologies".to_string(),
            "Identify dependencies".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("design", "Detailed Design", "Document component design", "architect")
            .with_actions(vec![
                "Design components".to_string(),
                "Define interfaces".to_string(),
                "Document data flow".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("risks", "Risk Assessment", "Identify and mitigate risks", "architect")
            .with_actions(vec![
                "List technical risks".to_string(),
                "Define mitigation strategies".to_string(),
                "Plan contingencies".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("review", "Technical Review", "Get technical approval", "architect")
            .with_actions(vec![
                "Review with team".to_string(),
                "Address concerns".to_string(),
                "Finalize spec".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn api_design() -> Workflow {
    Workflow::new(
        "api-design",
        "API Design",
        WorkflowPhase::Planning,
        "Design RESTful or GraphQL API",
    )
    .with_step(
        WorkflowStep::new("resources", "Define Resources", "Identify API resources", "architect")
            .with_actions(vec![
                "List resources".to_string(),
                "Define relationships".to_string(),
                "Plan URL structure".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("endpoints", "Design Endpoints", "Define API endpoints", "architect")
            .with_actions(vec![
                "Define CRUD operations".to_string(),
                "Specify request/response formats".to_string(),
                "Document error handling".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("security", "Security Design", "Plan API security", "security")
            .with_actions(vec![
                "Define authentication".to_string(),
                "Plan authorization".to_string(),
                "Document rate limiting".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("documentation", "API Documentation", "Create API docs", "tech-writer")
            .with_actions(vec![
                "Write OpenAPI spec".to_string(),
                "Add examples".to_string(),
                "Generate documentation".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

// ============ Solutioning Phase ============

pub fn architecture() -> Workflow {
    Workflow::new(
        "architecture",
        "Architecture Design",
        WorkflowPhase::Solutioning,
        "Design system architecture",
    )
    .with_step(
        WorkflowStep::new("context", "System Context", "Define system boundaries", "architect")
            .with_actions(vec![
                "Identify external systems".to_string(),
                "Define integration points".to_string(),
                "Document data flows".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new(
            "containers",
            "Container Design",
            "Design high-level components",
            "architect",
        )
        .with_actions(vec![
            "Identify containers".to_string(),
            "Define responsibilities".to_string(),
            "Plan communication".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new(
            "components",
            "Component Design",
            "Design internal components",
            "architect",
        )
        .with_actions(vec![
            "Break down containers".to_string(),
            "Define interfaces".to_string(),
            "Plan dependencies".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("decisions", "Architecture Decisions", "Document ADRs", "architect")
            .with_actions(vec![
                "Document key decisions".to_string(),
                "Explain rationale".to_string(),
                "Note alternatives considered".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn epics_and_stories() -> Workflow {
    Workflow::new(
        "epics-and-stories",
        "Epics and Stories",
        WorkflowPhase::Solutioning,
        "Break down work into epics and user stories",
    )
    .with_step(
        WorkflowStep::new("epics", "Define Epics", "Create high-level epics", "pm").with_actions(
            vec![
                "Identify major features".to_string(),
                "Group related functionality".to_string(),
                "Prioritize epics".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("stories", "Write User Stories", "Break epics into stories", "pm")
            .with_actions(vec![
                "Write user stories".to_string(),
                "Define acceptance criteria".to_string(),
                "Estimate complexity".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("refine", "Refine Backlog", "Groom and prioritize", "scrum-master")
            .with_actions(vec![
                "Review with team".to_string(),
                "Clarify requirements".to_string(),
                "Update estimates".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn implementation_readiness() -> Workflow {
    Workflow::new(
        "implementation-readiness",
        "Implementation Readiness",
        WorkflowPhase::Solutioning,
        "Ensure team is ready to implement",
    )
    .with_step(
        WorkflowStep::new(
            "checklist",
            "Readiness Checklist",
            "Verify prerequisites",
            "scrum-master",
        )
        .with_actions(vec![
            "Check requirements clarity".to_string(),
            "Verify design completeness".to_string(),
            "Confirm resource availability".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new(
            "environment",
            "Environment Setup",
            "Prepare development environment",
            "devops",
        )
        .with_actions(vec![
            "Set up repositories".to_string(),
            "Configure CI/CD".to_string(),
            "Prepare test environments".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("kickoff", "Sprint Kickoff", "Start implementation", "scrum-master")
            .with_actions(vec![
                "Review sprint goals".to_string(),
                "Assign initial tasks".to_string(),
                "Set communication cadence".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn data_model() -> Workflow {
    Workflow::new(
        "data-model",
        "Data Model Design",
        WorkflowPhase::Solutioning,
        "Design database schema and data model",
    )
    .with_step(
        WorkflowStep::new("entities", "Define Entities", "Identify data entities", "data-engineer")
            .with_actions(vec![
                "List entities".to_string(),
                "Define attributes".to_string(),
                "Identify relationships".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("schema", "Design Schema", "Create database schema", "data-engineer")
            .with_actions(vec![
                "Design tables".to_string(),
                "Define indexes".to_string(),
                "Plan partitioning".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new(
            "migration",
            "Migration Strategy",
            "Plan data migration",
            "data-engineer",
        )
        .with_actions(vec![
            "Design migration scripts".to_string(),
            "Plan rollback strategy".to_string(),
            "Test migration".to_string(),
        ])
        .as_checkpoint(),
    )
    .as_builtin()
}

// ============ Implementation Phase ============

pub fn sprint_planning() -> Workflow {
    Workflow::new(
        "sprint-planning",
        "Sprint Planning",
        WorkflowPhase::Implementation,
        "Plan sprint work and commitments",
    )
    .with_step(
        WorkflowStep::new(
            "review-backlog",
            "Review Backlog",
            "Review prioritized backlog",
            "scrum-master",
        )
        .with_actions(vec![
            "Review sprint goal".to_string(),
            "Discuss top priorities".to_string(),
            "Clarify requirements".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("capacity", "Assess Capacity", "Determine team capacity", "scrum-master")
            .with_actions(vec![
                "Calculate available hours".to_string(),
                "Account for meetings".to_string(),
                "Consider PTO".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("commit", "Sprint Commitment", "Commit to sprint work", "scrum-master")
            .with_actions(vec![
                "Select stories".to_string(),
                "Break into tasks".to_string(),
                "Confirm commitment".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn dev_story() -> Workflow {
    Workflow::new(
        "dev-story",
        "Development Story",
        WorkflowPhase::Implementation,
        "Implement a user story",
    )
    .with_step(
        WorkflowStep::new("understand", "Understand Story", "Review requirements", "developer")
            .with_actions(vec![
                "Read acceptance criteria".to_string(),
                "Ask clarifying questions".to_string(),
                "Plan approach".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("implement", "Implement", "Write code", "developer").with_actions(vec![
            "Create feature branch".to_string(),
            "Write implementation".to_string(),
            "Write tests".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("test", "Test", "Verify implementation", "developer").with_actions(vec![
            "Run unit tests".to_string(),
            "Manual testing".to_string(),
            "Fix issues".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("pr", "Create PR", "Submit for review", "developer")
            .with_actions(vec![
                "Create pull request".to_string(),
                "Add description".to_string(),
                "Request review".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn code_review() -> Workflow {
    Workflow::new(
        "code-review",
        "Code Review",
        WorkflowPhase::Implementation,
        "Review code changes",
    )
    .with_step(
        WorkflowStep::new("context", "Understand Context", "Review PR description", "reviewer")
            .with_actions(vec![
                "Read PR description".to_string(),
                "Check linked issues".to_string(),
                "Understand goal".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("review", "Review Code", "Examine changes", "reviewer").with_actions(
            vec![
                "Check correctness".to_string(),
                "Check security".to_string(),
                "Check performance".to_string(),
                "Check readability".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("feedback", "Provide Feedback", "Give constructive feedback", "reviewer")
            .with_actions(vec![
                "Praise good patterns".to_string(),
                "Suggest improvements".to_string(),
                "Approve or request changes".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn retrospective() -> Workflow {
    Workflow::new(
        "retrospective",
        "Sprint Retrospective",
        WorkflowPhase::Implementation,
        "Reflect on sprint and improve",
    )
    .with_step(
        WorkflowStep::new("gather", "Gather Feedback", "Collect team input", "scrum-master")
            .with_actions(vec![
                "What went well".to_string(),
                "What didn't go well".to_string(),
                "What to improve".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("discuss", "Discuss", "Analyze feedback", "scrum-master").with_actions(
            vec![
                "Group similar items".to_string(),
                "Identify root causes".to_string(),
                "Prioritize issues".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new(
            "actions",
            "Define Actions",
            "Create improvement actions",
            "scrum-master",
        )
        .with_actions(vec![
            "Define action items".to_string(),
            "Assign owners".to_string(),
            "Set deadlines".to_string(),
        ])
        .as_checkpoint(),
    )
    .as_builtin()
}

pub fn sprint_status() -> Workflow {
    Workflow::new(
        "sprint-status",
        "Sprint Status",
        WorkflowPhase::Implementation,
        "Report sprint progress",
    )
    .with_step(
        WorkflowStep::new("metrics", "Gather Metrics", "Collect sprint data", "scrum-master")
            .with_actions(vec![
                "Calculate velocity".to_string(),
                "Track burndown".to_string(),
                "Note blockers".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("report", "Create Report", "Summarize status", "scrum-master")
            .with_actions(vec![
                "Write summary".to_string(),
                "Highlight risks".to_string(),
                "Note achievements".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

// ============ Quick Flow ============

pub fn quick_bug_fix() -> Workflow {
    Workflow::new("quick-bug-fix", "Quick Bug Fix", WorkflowPhase::QuickFlow, "Rapidly fix a bug")
        .with_step(
            WorkflowStep::new("reproduce", "Reproduce", "Confirm the bug", "developer")
                .with_actions(vec![
                    "Read bug report".to_string(),
                    "Reproduce issue".to_string(),
                    "Identify root cause".to_string(),
                ]),
        )
        .with_step(WorkflowStep::new("fix", "Fix", "Implement fix", "developer").with_actions(
            vec![
                "Write failing test".to_string(),
                "Implement fix".to_string(),
                "Verify test passes".to_string(),
            ],
        ))
        .with_step(
            WorkflowStep::new("verify", "Verify", "Confirm fix works", "developer")
                .with_actions(vec![
                    "Run all tests".to_string(),
                    "Manual verification".to_string(),
                    "Create PR".to_string(),
                ])
                .as_checkpoint(),
        )
        .as_builtin()
}

pub fn quick_feature() -> Workflow {
    Workflow::new(
        "quick-feature",
        "Quick Feature",
        WorkflowPhase::QuickFlow,
        "Rapidly implement a small feature",
    )
    .with_step(
        WorkflowStep::new("understand", "Understand", "Clarify requirements", "developer")
            .with_actions(vec![
                "Review requirements".to_string(),
                "Ask questions".to_string(),
                "Plan approach".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("implement", "Implement", "Build feature", "developer").with_actions(
            vec![
                "Write code".to_string(),
                "Write tests".to_string(),
                "Update docs".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("ship", "Ship", "Deploy feature", "developer")
            .with_actions(vec![
                "Create PR".to_string(),
                "Get review".to_string(),
                "Merge and deploy".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn quick_refactor() -> Workflow {
    Workflow::new(
        "quick-refactor",
        "Quick Refactor",
        WorkflowPhase::QuickFlow,
        "Safely refactor code",
    )
    .with_step(
        WorkflowStep::new("tests", "Ensure Tests", "Verify test coverage", "developer")
            .with_actions(vec![
                "Check coverage".to_string(),
                "Add missing tests".to_string(),
                "Run baseline".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("refactor", "Refactor", "Make changes", "developer").with_actions(vec![
            "Small incremental changes".to_string(),
            "Run tests after each".to_string(),
            "Commit frequently".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("verify", "Verify", "Confirm behavior unchanged", "developer")
            .with_actions(vec![
                "Run all tests".to_string(),
                "Compare before/after".to_string(),
                "Create PR".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

// ============ Testing ============

pub fn test_design() -> Workflow {
    Workflow::new(
        "test-design",
        "Test Design",
        WorkflowPhase::Testing,
        "Design test strategy and cases",
    )
    .with_step(
        WorkflowStep::new("strategy", "Test Strategy", "Define testing approach", "test-architect")
            .with_actions(vec![
                "Define test levels".to_string(),
                "Identify test types".to_string(),
                "Plan coverage".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("cases", "Test Cases", "Write test cases", "test-architect")
            .with_actions(vec![
                "Write test scenarios".to_string(),
                "Define test data".to_string(),
                "Prioritize tests".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn test_automation() -> Workflow {
    Workflow::new(
        "test-automation",
        "Test Automation",
        WorkflowPhase::Testing,
        "Automate test execution",
    )
    .with_step(
        WorkflowStep::new(
            "framework",
            "Setup Framework",
            "Configure test framework",
            "test-architect",
        )
        .with_actions(vec![
            "Select framework".to_string(),
            "Configure environment".to_string(),
            "Set up CI integration".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("implement", "Implement Tests", "Write automated tests", "developer")
            .with_actions(vec![
                "Write unit tests".to_string(),
                "Write integration tests".to_string(),
                "Write e2e tests".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("maintain", "Maintain", "Keep tests healthy", "test-architect")
            .with_actions(vec![
                "Fix flaky tests".to_string(),
                "Update test data".to_string(),
                "Improve coverage".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn test_review() -> Workflow {
    Workflow::new(
        "test-review",
        "Test Review",
        WorkflowPhase::Testing,
        "Review test quality and coverage",
    )
    .with_step(
        WorkflowStep::new("coverage", "Review Coverage", "Analyze test coverage", "test-architect")
            .with_actions(vec![
                "Generate coverage report".to_string(),
                "Identify gaps".to_string(),
                "Prioritize additions".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("quality", "Review Quality", "Assess test quality", "test-architect")
            .with_actions(vec![
                "Check test isolation".to_string(),
                "Review assertions".to_string(),
                "Identify improvements".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

// ============ Documentation ============

pub fn document_project() -> Workflow {
    Workflow::new(
        "document-project",
        "Document Project",
        WorkflowPhase::Documentation,
        "Create project documentation",
    )
    .with_step(
        WorkflowStep::new("readme", "README", "Write project README", "tech-writer").with_actions(
            vec![
                "Write overview".to_string(),
                "Add installation".to_string(),
                "Include examples".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("guides", "User Guides", "Write user documentation", "tech-writer")
            .with_actions(vec![
                "Write getting started".to_string(),
                "Document features".to_string(),
                "Add tutorials".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("reference", "Reference", "Write reference docs", "tech-writer")
            .with_actions(vec![
                "Document APIs".to_string(),
                "Add configuration".to_string(),
                "Include troubleshooting".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn api_documentation() -> Workflow {
    Workflow::new(
        "api-documentation",
        "API Documentation",
        WorkflowPhase::Documentation,
        "Document API endpoints",
    )
    .with_step(
        WorkflowStep::new("spec", "API Spec", "Write OpenAPI spec", "tech-writer").with_actions(
            vec![
                "Document endpoints".to_string(),
                "Define schemas".to_string(),
                "Add examples".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("generate", "Generate Docs", "Generate documentation", "tech-writer")
            .with_actions(vec![
                "Generate from spec".to_string(),
                "Add descriptions".to_string(),
                "Include code samples".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

// ============ DevOps ============

pub fn ci_setup() -> Workflow {
    Workflow::new("ci-setup", "CI Setup", WorkflowPhase::DevOps, "Set up continuous integration")
        .with_step(
            WorkflowStep::new("pipeline", "Define Pipeline", "Create CI pipeline", "devops")
                .with_actions(vec![
                    "Define stages".to_string(),
                    "Configure triggers".to_string(),
                    "Set up caching".to_string(),
                ]),
        )
        .with_step(
            WorkflowStep::new("quality", "Quality Gates", "Add quality checks", "devops")
                .with_actions(vec![
                    "Add linting".to_string(),
                    "Add testing".to_string(),
                    "Add security scans".to_string(),
                ]),
        )
        .with_step(
            WorkflowStep::new("artifacts", "Artifacts", "Configure artifacts", "devops")
                .with_actions(vec![
                    "Build artifacts".to_string(),
                    "Store artifacts".to_string(),
                    "Version artifacts".to_string(),
                ])
                .as_checkpoint(),
        )
        .as_builtin()
}

pub fn deployment() -> Workflow {
    Workflow::new("deployment", "Deployment", WorkflowPhase::DevOps, "Deploy to production")
        .with_step(
            WorkflowStep::new("prepare", "Prepare", "Prepare for deployment", "devops")
                .with_actions(vec![
                    "Verify build".to_string(),
                    "Check dependencies".to_string(),
                    "Review changes".to_string(),
                ]),
        )
        .with_step(
            WorkflowStep::new("deploy", "Deploy", "Execute deployment", "devops").with_actions(
                vec![
                    "Deploy to staging".to_string(),
                    "Run smoke tests".to_string(),
                    "Deploy to production".to_string(),
                ],
            ),
        )
        .with_step(
            WorkflowStep::new("verify", "Verify", "Verify deployment", "devops")
                .with_actions(vec![
                    "Check health".to_string(),
                    "Monitor metrics".to_string(),
                    "Confirm success".to_string(),
                ])
                .as_checkpoint(),
        )
        .as_builtin()
}

// ============ Additional ============

pub fn security_review() -> Workflow {
    Workflow::new(
        "security-review",
        "Security Review",
        WorkflowPhase::Implementation,
        "Review code for security issues",
    )
    .with_step(
        WorkflowStep::new("scan", "Security Scan", "Run security scanners", "security")
            .with_actions(vec![
                "Run SAST".to_string(),
                "Run dependency scan".to_string(),
                "Check secrets".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("review", "Manual Review", "Review findings", "security").with_actions(
            vec![
                "Triage findings".to_string(),
                "Verify vulnerabilities".to_string(),
                "Prioritize fixes".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("remediate", "Remediate", "Fix issues", "developer")
            .with_actions(vec![
                "Fix vulnerabilities".to_string(),
                "Update dependencies".to_string(),
                "Verify fixes".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn performance_review() -> Workflow {
    Workflow::new(
        "performance-review",
        "Performance Review",
        WorkflowPhase::Implementation,
        "Review and optimize performance",
    )
    .with_step(
        WorkflowStep::new("profile", "Profile", "Profile application", "performance").with_actions(
            vec![
                "Run profiler".to_string(),
                "Identify hotspots".to_string(),
                "Measure baselines".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("optimize", "Optimize", "Implement optimizations", "performance")
            .with_actions(vec![
                "Optimize algorithms".to_string(),
                "Reduce allocations".to_string(),
                "Improve caching".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("verify", "Verify", "Verify improvements", "performance")
            .with_actions(vec![
                "Run benchmarks".to_string(),
                "Compare results".to_string(),
                "Document changes".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn onboarding() -> Workflow {
    Workflow::new(
        "onboarding",
        "Developer Onboarding",
        WorkflowPhase::Documentation,
        "Onboard new team members",
    )
    .with_step(
        WorkflowStep::new("setup", "Environment Setup", "Set up development environment", "mentor")
            .with_actions(vec![
                "Install tools".to_string(),
                "Clone repositories".to_string(),
                "Configure IDE".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("overview", "Project Overview", "Learn the codebase", "mentor")
            .with_actions(vec![
                "Architecture overview".to_string(),
                "Key components".to_string(),
                "Development workflow".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("first-task", "First Task", "Complete first contribution", "mentor")
            .with_actions(vec![
                "Pick starter task".to_string(),
                "Implement with guidance".to_string(),
                "Submit PR".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn tech_debt() -> Workflow {
    Workflow::new(
        "tech-debt",
        "Tech Debt Reduction",
        WorkflowPhase::Implementation,
        "Address technical debt",
    )
    .with_step(
        WorkflowStep::new("identify", "Identify Debt", "Find technical debt", "architect")
            .with_actions(vec![
                "Review code quality".to_string(),
                "Check dependencies".to_string(),
                "Assess architecture".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("prioritize", "Prioritize", "Rank debt items", "architect").with_actions(
            vec![
                "Assess impact".to_string(),
                "Estimate effort".to_string(),
                "Create backlog".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("address", "Address Debt", "Fix debt items", "developer")
            .with_actions(vec![
                "Refactor code".to_string(),
                "Update dependencies".to_string(),
                "Improve tests".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn migration() -> Workflow {
    Workflow::new(
        "migration",
        "System Migration",
        WorkflowPhase::Implementation,
        "Migrate to new system or version",
    )
    .with_step(
        WorkflowStep::new("plan", "Plan Migration", "Create migration plan", "architect")
            .with_actions(vec![
                "Assess current state".to_string(),
                "Define target state".to_string(),
                "Plan phases".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("execute", "Execute Migration", "Perform migration", "developer")
            .with_actions(vec![
                "Migrate data".to_string(),
                "Update code".to_string(),
                "Test thoroughly".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("validate", "Validate", "Verify migration success", "test-architect")
            .with_actions(vec![
                "Run validation tests".to_string(),
                "Compare data".to_string(),
                "Confirm functionality".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn incident_response() -> Workflow {
    Workflow::new(
        "incident-response",
        "Incident Response",
        WorkflowPhase::DevOps,
        "Respond to production incidents",
    )
    .with_step(
        WorkflowStep::new("triage", "Triage", "Assess incident severity", "devops").with_actions(
            vec![
                "Identify impact".to_string(),
                "Assign severity".to_string(),
                "Notify stakeholders".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("mitigate", "Mitigate", "Reduce impact", "devops").with_actions(vec![
            "Implement workaround".to_string(),
            "Scale resources".to_string(),
            "Communicate status".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("resolve", "Resolve", "Fix root cause", "developer").with_actions(vec![
            "Identify root cause".to_string(),
            "Implement fix".to_string(),
            "Deploy fix".to_string(),
        ]),
    )
    .with_step(
        WorkflowStep::new("postmortem", "Postmortem", "Learn from incident", "scrum-master")
            .with_actions(vec![
                "Document timeline".to_string(),
                "Identify improvements".to_string(),
                "Create action items".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

pub fn release_planning() -> Workflow {
    Workflow::new(
        "release-planning",
        "Release Planning",
        WorkflowPhase::Planning,
        "Plan product release",
    )
    .with_step(
        WorkflowStep::new("scope", "Define Scope", "Determine release scope", "pm").with_actions(
            vec![
                "Review backlog".to_string(),
                "Select features".to_string(),
                "Define milestones".to_string(),
            ],
        ),
    )
    .with_step(
        WorkflowStep::new("schedule", "Create Schedule", "Plan release timeline", "pm")
            .with_actions(vec![
                "Set release date".to_string(),
                "Plan sprints".to_string(),
                "Identify dependencies".to_string(),
            ]),
    )
    .with_step(
        WorkflowStep::new("communicate", "Communicate", "Share release plan", "pm")
            .with_actions(vec![
                "Create release notes".to_string(),
                "Notify stakeholders".to_string(),
                "Update roadmap".to_string(),
            ])
            .as_checkpoint(),
    )
    .as_builtin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_workflows_count() {
        let workflows = all_workflows();
        assert!(workflows.len() >= 30, "Expected at least 30 workflows, got {}", workflows.len());
    }

    #[test]
    fn test_all_workflows_have_unique_ids() {
        let workflows = all_workflows();
        let mut ids: Vec<&str> = workflows.iter().map(|w| w.id.as_str()).collect();
        ids.sort();
        let original_len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Duplicate workflow IDs found");
    }

    #[test]
    fn test_all_workflows_are_builtin() {
        let workflows = all_workflows();
        for workflow in workflows {
            assert!(workflow.builtin, "Workflow {} should be marked as builtin", workflow.id);
        }
    }

    #[test]
    fn test_all_workflows_have_steps() {
        let workflows = all_workflows();
        for workflow in workflows {
            assert!(!workflow.steps.is_empty(), "Workflow {} has no steps", workflow.id);
        }
    }

    #[test]
    fn test_all_workflows_have_checkpoints() {
        let workflows = all_workflows();
        for workflow in workflows {
            assert!(
                !workflow.checkpoints.is_empty(),
                "Workflow {} has no checkpoints",
                workflow.id
            );
        }
    }
}
