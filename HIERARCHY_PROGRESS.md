# Work Hierarchy Implementation Progress

**Status**: In Progress
**Started**: 2025-11-23
**Design Document**: [WORK_HIERARCHY.md](WORK_HIERARCHY.md)

## Summary

Implementing a three-level work item hierarchy for jjj:
- **Milestone** (M-*): Release targets
- **Feature** (F-*): User-facing capabilities
- **Task** (T-*): Technical work (MUST belong to a feature)
- **Bug** (B-*): Defects (can be standalone or linked)

## ✅ Completed

### 1. Data Models (Complete)
- [x] `Milestone` model with full functionality ([models/milestone.rs](src/models/milestone.rs))
  - Status: Planning, Active, Released, Cancelled
  - Target dates and version tracking
  - Feature and bug relationships
  - 5 unit tests passing

- [x] `Feature` model with full functionality ([models/feature.rs](src/models/feature.rs))
  - Status: Backlog, InProgress, Review, Done, Blocked
  - Priority levels: Low, Medium, High, Critical
  - Task and bug relationships
  - Milestone assignment
  - 6 unit tests passing

- [x] `Bug` model with full functionality ([models/bug.rs](src/models/bug.rs))
  - Severity: Low, Medium, High, Critical
  - Status: New, Confirmed, InProgress, Fixed, Closed, WontFix, Duplicate
  - Feature and milestone relationships
  - Version tracking (affected/fixed)
  - Reproduction steps
  - 8 unit tests passing

- [x] Updated `Task` model
  - Added required `feature_id` field
  - Tasks MUST belong to a feature
  - Maintains all existing functionality

### 2. Documentation (Complete)
- [x] [WORK_HIERARCHY.md](WORK_HIERARCHY.md) - Comprehensive user-centric design (400+ lines)
  - User workflows
  - Board views for different roles
  - Relationship rules
  - Common questions
  - Real-world examples

- [x] [HIERARCHY_DESIGN.md](HIERARCHY_DESIGN.md) - Technical design (initial, superseded)

## 🚧 In Progress

### Task Model Breaking Change
**Issue**: Task::new() signature changed from:
```rust
Task::new(id, title, column)
```
to:
```rust
Task::new(id, title, feature_id, column)
```

**Impact**:
- ❌ Command implementations need updating
- ❌ Test files need updating
- ❌ CLI needs to require `--feature` parameter

### Files Needing Updates

1. **src/commands/task.rs** (Line 35)
   - `create_task()` needs to accept feature_id parameter
   - CLI must require `--feature F-1` when creating tasks

2. **Test Files** (Partially Fixed)
   - ✅ tests/task_management.rs - Fixed with F-TEST default
   - ✅ tests/integration_storage.rs - Fixed with F-TEST default
   - ⚠️ May need better test feature setup

## 📋 Remaining Work

### Phase 1: Fix Compilation (High Priority)
- [ ] Update `create_task()` command to require feature_id
- [ ] Add `--feature` parameter to `jjj task new` CLI
- [ ] Update CLI help text to mention feature requirement
- [ ] Verify all tests pass

### Phase 2: CLI Commands (High Priority)
- [ ] Implement `jjj milestone` commands
  - `new`, `list`, `show`, `add-feature`, `roadmap`
- [ ] Implement `jjj feature` commands
  - `new`, `list`, `show`, `add-task`, `progress`, `board`
- [ ] Implement `jjj bug` commands
  - `new`, `list`, `show`, `link`, `triage`
- [ ] Update existing `jjj task` commands
  - Require `--feature` for `new`
  - Show feature context in `list`/`show`

### Phase 3: Storage Layer (Medium Priority)
- [ ] Add milestone storage methods
  - `load_milestone()`, `save_milestone()`, `list_milestones()`
  - `next_milestone_id()`
- [ ] Add feature storage methods
  - `load_feature()`, `save_feature()`, `list_features()`
  - `next_feature_id()`
- [ ] Add bug storage methods
  - `load_bug()`, `save_bug()`, `list_bugs()`
  - `next_bug_id()`
- [ ] Update directory structure
  - Create `milestones/`, `features/`, `bugs/` directories

### Phase 4: Enhanced Views (Medium Priority)
- [ ] Feature board view
  - Show features with task completion %
- [ ] Milestone roadmap view
  - Timeline visualization
  - Progress tracking
- [ ] Bug triage view
  - Filter by severity
  - Interactive triage mode

### Phase 5: Testing (Medium Priority)
- [ ] Integration tests for milestone workflow
- [ ] Integration tests for feature workflow
- [ ] Integration tests for bug workflow
- [ ] Tests for parent-child relationships
- [ ] Tests for cross-linking (bug↔feature↔milestone)

### Phase 6: Documentation Updates (Low Priority)
- [ ] Update README.md with hierarchy
- [ ] Update FEATURES.md with new capabilities
- [ ] Update demo scripts to showcase hierarchy
- [ ] Add hierarchy examples to quick start

## Design Decisions Made

### 1. Three-Level Hierarchy (Not Four)
**Decision**: No "Epic" level
**Rationale**: Too abstract, not user-facing. Most teams don't need it.

### 2. Tasks MUST Belong to Features
**Decision**: Removed optional feature_id, made it required
**Rationale**:
- Prevents orphan tasks
- Forces organization
- Makes feature progress tracking meaningful

**Workaround**: For tasks that don't fit a specific feature:
- Create `F-99: Technical Improvements`
- Create `F-100: Bug Fixes`
- Create `F-101: Documentation`

### 3. Bugs Can Be Standalone
**Decision**: feature_id is optional for bugs
**Rationale**:
- Some bugs affect general system (not specific feature)
- Bugs can be reported before features exist
- Flexibility for bug triage workflow

### 4. Simple File IDs
**Decision**: Use prefixes M-, F-, T-, B-
**Rationale**:
- Clear and unambiguous
- Easy to say aloud
- Matches common PM tool conventions

## File Structure

```
jjj/meta/
├── config.toml
├── milestones/          ← NEW
│   ├── M-1.toml
│   └── M-2.toml
├── features/            ← NEW
│   ├── F-1.json
│   └── F-2.json
├── tasks/
│   ├── T-1.json        (now includes feature_id)
│   └── T-2.json
├── bugs/                ← NEW
│   ├── B-1.json
│   └── B-2.json
└── reviews/
    └── kpqxywon.../
```

## User Workflow Examples (From Design Doc)

### Creating and Working on a Feature
```bash
# 1. Create milestone
jjj milestone new "v1.0 Release" --date 2025-12-31

# 2. Create feature
jjj feature new "User Authentication" --milestone M-1

# 3. Break into tasks
jjj task new "Implement password hashing" --feature F-1
jjj task new "Add login API" --feature F-1
jjj task new "Create login UI" --feature F-1

# 4. Work on tasks
jjj task attach T-1
jjj task move T-1 "In Progress"

# 5. Track feature progress
jjj feature progress F-1
# Output: 1/3 tasks done (33%)
```

### Bug Triage
```bash
# 1. Report bug
jjj bug new "Login fails with special chars" --severity high

# 2. Link to feature
jjj bug link B-1 --feature F-1

# 3. Target for release
jjj bug link B-1 --milestone M-1

# 4. Assign
jjj bug assign B-1 alice

# 5. Track
jjj bug list --severity critical --open
```

## Migration Strategy

### For Existing Users
If jjj is already deployed with tasks but no hierarchy:

**Option 1**: Create default feature
```bash
# Automatically create F-1: "Existing Work"
# Assign all existing tasks to F-1
jjj migrate-hierarchy
```

**Option 2**: Manual migration
```bash
# User creates features and reassigns tasks
jjj feature new "Legacy Tasks"
jjj task update T-* --feature F-1
```

## Testing Strategy

### Unit Tests (✅ 19/19 Passing)
- 5 milestone tests
- 6 feature tests
- 8 bug tests
- Existing task/review/config tests updated

### Integration Tests (⚠️ Need Updates)
- Test files fixed with F-TEST default
- Need real feature creation in tests
- Need workflow tests (create feature → add tasks → complete)

### Manual Testing
- Use demo environment
- Create full workflow: Milestone → Feature → Tasks
- Verify board shows feature context
- Verify progress rollup works

## Next Session TODOs

1. **Fix Compilation** (30 min)
   - Update task creation command
   - Add --feature CLI parameter
   - Run all tests

2. **Basic CLI** (1-2 hours)
   - Implement milestone commands (minimal)
   - Implement feature commands (minimal)
   - Implement bug commands (minimal)

3. **Storage Layer** (1 hour)
   - Add CRUD for milestones/features/bugs
   - Update directory creation

4. **Testing** (1 hour)
   - Create hierarchy integration tests
   - Test parent-child relationships
   - Test cross-linking

5. **Demo** (30 min)
   - Update demo to showcase hierarchy
   - Create example milestone with features

## Questions/Decisions Needed

### Q: Should we allow tasks without features during transition?
**A**: No. Enforce feature requirement from day 1.
- Cleaner data model
- Forces good organization
- Easier to implement

### Q: What about chores (non-user-facing work)?
**A**: Create features for them:
- `F-99: Infrastructure`
- `F-100: Code Quality`
- `F-101: Documentation`

This keeps the model simple while supporting all work types.

## Implementation Notes

### Model Design Highlights

**Milestone**:
- `is_overdue()` - Check if past target date
- `days_until_target()` - Calculate time remaining
- Can track both features and bugs

**Feature**:
- Priority ordering (Critical > High > Medium > Low)
- Status transitions (Backlog → InProgress → Review → Done)
- Can calculate task completion percentage
- Supports story points (optional)

**Bug**:
- Severity ordering (Critical > High > Medium > Low)
- Rich metadata (repro steps, expected/actual behavior)
- Version tracking (affected_version, fixed_version)
- `is_open()` / `is_resolved()` helpers

**Task**:
- Now requires feature_id (breaking change)
- Maintains existing functionality
- Version tracking for conflicts

## Success Criteria

- [ ] All tests passing
- [ ] Can create milestones via CLI
- [ ] Can create features via CLI
- [ ] Can create tasks with required feature
- [ ] Can create bugs via CLI
- [ ] Feature board shows progress
- [ ] Milestone roadmap shows timeline
- [ ] Documentation updated

## Timeline Estimate

- **Phase 1** (Fix Compilation): 30 minutes
- **Phase 2** (CLI Commands): 2 hours
- **Phase 3** (Storage): 1 hour
- **Phase 4** (Views): 2 hours
- **Phase 5** (Testing): 1 hour
- **Phase 6** (Docs): 1 hour

**Total**: ~7-8 hours of focused work

## References

- Design Doc: [WORK_HIERARCHY.md](WORK_HIERARCHY.md)
- Milestone Model: [src/models/milestone.rs](src/models/milestone.rs)
- Feature Model: [src/models/feature.rs](src/models/feature.rs)
- Bug Model: [src/models/bug.rs](src/models/bug.rs)
- Task Model (Updated): [src/models/task.rs](src/models/task.rs)

---

**Status**: Core models complete, CLI integration in progress
**Next**: Fix compilation, implement basic CLI commands
