-- Fixture: grpc_testing
-- Covers all four gRPC RPCs: CheckMembership, ListGroupMembers,
-- BatchListGroupMembers, GetGroupMembershipVersion

-- === Groups ===
INSERT INTO chat_group (group_id) VALUES
    ('a0000000-0000-0000-0000-000000000001'),  -- group with 3 members
    ('b0000000-0000-0000-0000-000000000002'),  -- group with 1 member (admin only)
    ('c0000000-0000-0000-0000-000000000003');  -- group with 2 members (shared user with group A)

-- === Memberships ===

-- Group A: 3 members (1 admin + 2 members)
INSERT INTO group_membership (group_id, user_id, role) VALUES
    ('a0000000-0000-0000-0000-000000000001', 'ee000000-0000-0000-0000-000000000001', 'admin'),
    ('a0000000-0000-0000-0000-000000000001', 'ee000000-0000-0000-0000-000000000002', 'member'),
    ('a0000000-0000-0000-0000-000000000001', 'ee000000-0000-0000-0000-000000000003', 'member');

-- Group B: admin only
INSERT INTO group_membership (group_id, user_id, role) VALUES
    ('b0000000-0000-0000-0000-000000000002', 'ee000000-0000-0000-0000-000000000004', 'admin');

-- Group C: 2 members, user_01 is also in group A (useful for CheckMembership)
INSERT INTO group_membership (group_id, user_id, role) VALUES
    ('c0000000-0000-0000-0000-000000000003', 'ee000000-0000-0000-0000-000000000001', 'admin'),
    ('c0000000-0000-0000-0000-000000000003', 'ee000000-0000-0000-0000-000000000005', 'member');

-- ============================================================
-- Test scenarios:
--
-- CheckMembership:
--   user_01 → true  (in groups A + C)
--   user_04 → true  (in group B)
--   ee000000-0000-0000-0000-999999999999 → false (not in any group)
--
-- ListGroupMembers:
--   group A → [user_01, user_02, user_03]
--   group B → [user_04]
--   group C → [user_01, user_05]
--
-- BatchListGroupMembers:
--   [group A, group B] → { A: [user_01..03], B: [user_04] }
--   [group A, group C] → { A: [user_01..03], C: [user_01, user_05] }
--
-- GetGroupMembershipVersion:
--   Each group starts at version 1 (from the trigger).
--   After inserting more members, version increments.
-- ============================================================
