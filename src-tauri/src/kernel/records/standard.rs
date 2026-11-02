//! The standard objects, declared here and synced into `object_meta` / `field_meta` at every
//! workspace open (as Twenty syncs its workspace entities). Missing rows are inserted; rows that
//! exist keep what the owner edited (labels, icons, options). Columns come from the migrations.

use anyhow::Result;
use sqlx::SqlitePool;

use super::types::{FieldOptions, FieldType, RelationKind, RelationOptions, SelectOption};

pub struct FieldDecl {
    pub name: &'static str,
    pub label: &'static str,
    pub r#type: FieldType,
    pub column: &'static str,
    pub options: FieldOptions,
    pub nullable: bool,
    pub unique: bool,
    pub system: bool,
    pub icon: &'static str,
    pub description: &'static str,
}

pub struct ObjectDecl {
    pub name_singular: &'static str,
    pub name_plural: &'static str,
    pub label_singular: &'static str,
    pub label_plural: &'static str,
    pub icon: &'static str,
    pub table: &'static str,
    pub system: bool,
    pub label_identifier: &'static str,
    pub image_identifier: Option<&'static str>,
    pub fields: Vec<FieldDecl>,
}

fn f(name: &'static str, label: &'static str, r#type: FieldType, column: &'static str, icon: &'static str) -> FieldDecl {
    FieldDecl { name, label, r#type, column, options: FieldOptions::default(), nullable: true, unique: false, system: false, icon, description: "" }
}

fn sys(mut d: FieldDecl) -> FieldDecl {
    d.system = true;
    d
}

fn required(mut d: FieldDecl) -> FieldDecl {
    d.nullable = false;
    d
}

fn to_one(name: &'static str, label: &'static str, column: &'static str, target: &'static str, inverse: &'static str, icon: &'static str) -> FieldDecl {
    let mut d = f(name, label, FieldType::Relation, column, icon);
    d.options.relation = Some(RelationOptions { target: target.into(), kind: RelationKind::ManyToOne, inverse: Some(inverse.into()), junction: false });
    d
}

fn to_many(name: &'static str, label: &'static str, target: &'static str, inverse: &'static str, icon: &'static str) -> FieldDecl {
    let mut d = f(name, label, FieldType::Relation, "", icon);
    d.options.relation = Some(RelationOptions { target: target.into(), kind: RelationKind::OneToMany, inverse: Some(inverse.into()), junction: false });
    d
}

fn select(name: &'static str, label: &'static str, column: &'static str, icon: &'static str, options: &[(&str, &str, &str)]) -> FieldDecl {
    let mut d = f(name, label, FieldType::Select, column, icon);
    d.options.select = options
        .iter()
        .enumerate()
        .map(|(i, (value, label, color))| SelectOption { value: (*value).into(), label: (*label).into(), color: (*color).into(), position: i as f64 })
        .collect();
    d
}

fn multi_select(name: &'static str, label: &'static str, column: &'static str, icon: &'static str, options: &[(&str, &str, &str)]) -> FieldDecl {
    let mut d = select(name, label, column, icon, options);
    d.r#type = FieldType::MultiSelect;
    d
}

/// The fields every record-bearing object shares, after its own.
fn common() -> Vec<FieldDecl> {
    vec![
        sys(f("position", "Position", FieldType::Position, "position", "IconHierarchy2")),
        sys(f("createdBy", "Created by", FieldType::Actor, "created_by", "IconCreativeCommonsSa")),
    ]
}

/// The relations every record-bearing object has to the system objects.
fn targets(inverse: &'static str) -> Vec<FieldDecl> {
    vec![
        sys(to_many("noteTargets", "Notes", "noteTarget", inverse, "IconNotes")),
        sys(to_many("taskTargets", "Tasks", "taskTarget", inverse, "IconCheckbox")),
        sys(to_many("attachments", "Attachments", "attachment", inverse, "IconFileImport")),
        sys(to_many("favorites", "Favorites", "favorite", inverse, "IconHeart")),
        sys(to_many("timelineActivities", "Timeline activities", "timelineActivity", inverse, "IconTimelineEvent")),
    ]
}

pub fn standard_objects() -> Vec<ObjectDecl> {
    let mut objects = Vec::new();

    let mut person = vec![
        required(f("name", "Name", FieldType::FullName, "name", "IconUser")),
        f("emails", "Emails", FieldType::Emails, "emails", "IconMail"),
        f("linkedinLink", "Linkedin", FieldType::Links, "linkedin", "IconBrandLinkedin"),
        f("xLink", "X", FieldType::Links, "x", "IconBrandX"),
        f("jobTitle", "Job title", FieldType::Text, "title", "IconBriefcase"),
        f("phones", "Phones", FieldType::Phones, "phones", "IconPhone"),
        f("city", "City", FieldType::Text, "city", "IconMap"),
        f("avatarUrl", "Avatar", FieldType::Text, "avatar_url", "IconFileUpload"),
        f("notes", "Notes", FieldType::LongText, "notes", "IconNotes"),
        to_one("company", "Company", "company", "company", "people", "IconBuildingSkyscraper"),
        to_many("pointOfContactForOpportunities", "Linked opportunities", "opportunity", "pointOfContact", "IconTargetArrow"),
    ];
    person.extend(common());
    person.extend(targets("person"));
    objects.push(ObjectDecl {
        name_singular: "person",
        name_plural: "people",
        label_singular: "Person",
        label_plural: "People",
        icon: "IconUser",
        table: "person",
        system: false,
        label_identifier: "name",
        image_identifier: Some("avatarUrl"),
        fields: person,
    });

    let mut company = vec![
        required(f("name", "Name", FieldType::Text, "name", "IconBuildingSkyscraper")),
        f("domainName", "Domain name", FieldType::Domain, "domain", "IconLink"),
        f("linkedinLink", "Linkedin", FieldType::Links, "linkedin", "IconBrandLinkedin"),
        f("xLink", "X", FieldType::Links, "x", "IconBrandX"),
        f("annualRevenue", "Annual revenue", FieldType::Currency, "annual_revenue", "IconMoneybag"),
        f("address", "Address", FieldType::Address, "address", "IconMap"),
        f("employees", "Employees", FieldType::Number, "employees", "IconUsers"),
        f("idealCustomerProfile", "ICP", FieldType::Boolean, "ideal_customer_profile", "IconTarget"),
        f("tagline", "Tagline", FieldType::Text, "tagline", "IconAdCircle"),
        multi_select("workPolicy", "Work policy", "work_policy", "IconHome", &[("ON_SITE", "On-Site", "green"), ("HYBRID", "Hybrid", "turquoise"), ("REMOTE_WORK", "Remote work", "sky")]),
        f("visaSponsorship", "Visa sponsorship", FieldType::Boolean, "visa_sponsorship", "IconBrandVisa"),
        to_one("accountOwner", "Account owner", "account_owner", "workspaceMember", "accountOwnerForCompanies", "IconUserCircle"),
        to_many("people", "People", "person", "company", "IconUsers"),
        to_many("opportunities", "Opportunities", "opportunity", "company", "IconTargetArrow"),
    ];
    company.extend(common());
    company.extend(targets("company"));
    objects.push(ObjectDecl {
        name_singular: "company",
        name_plural: "companies",
        label_singular: "Company",
        label_plural: "Companies",
        icon: "IconBuildingSkyscraper",
        table: "company",
        system: false,
        label_identifier: "name",
        image_identifier: None,
        fields: company,
    });

    let mut opportunity = vec![
        required(f("name", "Name", FieldType::Text, "name", "IconTargetArrow")),
        f("amount", "Amount", FieldType::Currency, "amount", "IconCurrencyDollar"),
        f("closeDate", "Close date", FieldType::Date, "close_date", "IconCalendarEvent"),
        required(select("stage", "Stage", "stage", "IconProgressCheck", &[
            ("NEW", "New", "red"),
            ("SCREENING", "Screening", "purple"),
            ("MEETING", "Meeting", "sky"),
            ("PROPOSAL", "Proposal", "turquoise"),
            ("CUSTOMER", "Customer", "yellow"),
        ])),
        f("probability", "Probability", FieldType::Number, "probability", "IconProgressCheck"),
        to_one("pointOfContact", "Point of contact", "point_of_contact", "person", "pointOfContactForOpportunities", "IconUser"),
        to_one("company", "Company", "company", "company", "opportunities", "IconBuildingSkyscraper"),
        to_one("owner", "Owner", "owner", "workspaceMember", "ownedOpportunities", "IconUserCircle"),
    ];
    opportunity.extend(common());
    opportunity.extend(targets("opportunity"));
    objects.push(ObjectDecl {
        name_singular: "opportunity",
        name_plural: "opportunities",
        label_singular: "Opportunity",
        label_plural: "Opportunities",
        icon: "IconTargetArrow",
        table: "opportunity",
        system: false,
        label_identifier: "name",
        image_identifier: None,
        fields: opportunity,
    });

    let mut note = vec![
        required(f("title", "Title", FieldType::Text, "title", "IconNotes")),
        f("body", "Body", FieldType::RichText, "body", "IconFilePencil"),
        to_many("noteTargets", "Relations", "noteTarget", "note", "IconArrowUpRight"),
        to_many("attachments", "Attachments", "attachment", "note", "IconFileImport"),
        to_many("favorites", "Favorites", "favorite", "note", "IconHeart"),
        to_many("timelineActivities", "Timeline activities", "timelineActivity", "note", "IconTimelineEvent"),
    ];
    note.extend(common());
    objects.push(ObjectDecl {
        name_singular: "note",
        name_plural: "notes",
        label_singular: "Note",
        label_plural: "Notes",
        icon: "IconNotes",
        table: "note",
        system: false,
        label_identifier: "title",
        image_identifier: None,
        fields: note,
    });

    objects.push(ObjectDecl {
        name_singular: "noteTarget",
        name_plural: "noteTargets",
        label_singular: "Note target",
        label_plural: "Note targets",
        icon: "IconCheckbox",
        table: "note_target",
        system: true,
        label_identifier: "note",
        image_identifier: None,
        fields: vec![
            sys(to_one("note", "Note", "note", "note", "noteTargets", "IconNotes")),
            sys(to_one("person", "Person", "person", "person", "noteTargets", "IconUser")),
            sys(to_one("company", "Company", "company", "company", "noteTargets", "IconBuildingSkyscraper")),
            sys(to_one("opportunity", "Opportunity", "opportunity", "opportunity", "noteTargets", "IconTargetArrow")),
        ],
    });

    let mut task = vec![
        required(f("title", "Title", FieldType::Text, "title", "IconNotes")),
        f("body", "Body", FieldType::RichText, "body", "IconFilePencil"),
        // Twenty's dueAt is a date and time; New Dawn's tasks keep a calendar day (D-042).
        f("dueAt", "Due date", FieldType::Date, "due_on", "IconCalendarEvent"),
        required(select("status", "Status", "status", "IconCheck", &[("TODO", "To do", "sky"), ("IN_PROGRESS", "In progress", "purple"), ("DONE", "Done", "green")])),
        to_one("assignee", "Assignee", "assignee", "workspaceMember", "assignedTasks", "IconUserCircle"),
        to_many("taskTargets", "Relations", "taskTarget", "task", "IconArrowUpRight"),
        to_many("attachments", "Attachments", "attachment", "task", "IconFileImport"),
        to_many("favorites", "Favorites", "favorite", "task", "IconHeart"),
        to_many("timelineActivities", "Timeline activities", "timelineActivity", "task", "IconTimelineEvent"),
    ];
    task.extend(common());
    objects.push(ObjectDecl {
        name_singular: "task",
        name_plural: "tasks",
        label_singular: "Task",
        label_plural: "Tasks",
        icon: "IconCheckbox",
        table: "task",
        system: false,
        label_identifier: "title",
        image_identifier: None,
        fields: task,
    });

    objects.push(ObjectDecl {
        name_singular: "taskTarget",
        name_plural: "taskTargets",
        label_singular: "Task target",
        label_plural: "Task targets",
        icon: "IconCheckbox",
        table: "task_target",
        system: true,
        label_identifier: "task",
        image_identifier: None,
        fields: vec![
            sys(to_one("task", "Task", "task", "task", "taskTargets", "IconCheckbox")),
            sys(to_one("person", "Person", "person", "person", "taskTargets", "IconUser")),
            sys(to_one("company", "Company", "company", "company", "taskTargets", "IconBuildingSkyscraper")),
            sys(to_one("opportunity", "Opportunity", "opportunity", "opportunity", "taskTargets", "IconTargetArrow")),
        ],
    });

    objects.push(ObjectDecl {
        name_singular: "workspaceMember",
        name_plural: "workspaceMembers",
        label_singular: "Workspace member",
        label_plural: "Workspace members",
        icon: "IconUserCircle",
        table: "workspace_member",
        system: true,
        label_identifier: "name",
        image_identifier: Some("avatarUrl"),
        fields: vec![
            required(f("name", "Name", FieldType::FullName, "name", "IconCircleUser")),
            f("colorScheme", "Color scheme", FieldType::Text, "color_scheme", "IconColorSwatch"),
            f("locale", "Language", FieldType::Text, "locale", "IconLanguage"),
            f("avatarUrl", "Avatar", FieldType::Text, "avatar_url", "IconFileUpload"),
            f("userEmail", "User email", FieldType::Text, "user_email", "IconMail"),
            f("timeZone", "Time zone", FieldType::Text, "time_zone", "IconTimezone"),
            f("dateFormat", "Date format", FieldType::Text, "date_format", "IconCalendarEvent"),
            f("timeFormat", "Time format", FieldType::Text, "time_format", "IconClock"),
            f("numberFormat", "Number format", FieldType::Text, "number_format", "IconNumbers"),
            f("calendarStartDay", "Calendar start day", FieldType::Text, "calendar_start_day", "IconCalendarWeek"),
            to_many("accountOwnerForCompanies", "Account owner for companies", "company", "accountOwner", "IconBuildingSkyscraper"),
            to_many("ownedOpportunities", "Owned opportunities", "opportunity", "owner", "IconTargetArrow"),
            to_many("assignedTasks", "Assigned tasks", "task", "assignee", "IconCheckbox"),
            to_many("authoredAttachments", "Authored attachments", "attachment", "author", "IconFileImport"),
            to_many("favorites", "Favorites", "favorite", "workspaceMember", "IconHeart"),
            to_many("timelineActivities", "Timeline activities", "timelineActivity", "workspaceMember", "IconTimelineEvent"),
        ],
    });

    // Dashboards (D-067, 0.10.10): records whose page is a layout of their own (page_layout kind
    // DASHBOARD); pinnable; no notes, tasks or files of their own.
    let mut dashboard = vec![
        required(f("name", "Name", FieldType::Text, "name", "IconLayoutDashboard")),
        sys(f("icon", "Icon", FieldType::Text, "icon", "IconPhoto")),
        sys(to_many("favorites", "Favorites", "favorite", "dashboard", "IconHeart")),
    ];
    dashboard.extend(common());
    objects.push(ObjectDecl {
        name_singular: "dashboard",
        name_plural: "dashboards",
        label_singular: "Dashboard",
        label_plural: "Dashboards",
        icon: "IconLayoutDashboard",
        table: "dashboard",
        system: false,
        label_identifier: "name",
        image_identifier: None,
        fields: dashboard,
    });

    // Workflows (D-068, 0.10.11): a workflow is a record whose page is the builder; a run is a
    // record whose page shows its steps. Their versions and triggers are the engine's own tables.
    let mut workflow = vec![
        required(f("name", "Name", FieldType::Text, "name", "IconSettingsAutomation")),
        required(select("status", "Status", "status", "IconCircleDot", &[("DRAFT", "Draft", "gray"), ("ACTIVE", "Active", "green"), ("DEACTIVATED", "Deactivated", "orange")])),
        sys(to_many("runs", "Runs", "workflowRun", "workflow", "IconHistory")),
    ];
    workflow.extend(common());
    objects.push(ObjectDecl {
        name_singular: "workflow",
        name_plural: "workflows",
        label_singular: "Workflow",
        label_plural: "Workflows",
        icon: "IconSettingsAutomation",
        table: "workflow",
        system: false,
        label_identifier: "name",
        image_identifier: None,
        fields: workflow,
    });
    let mut run = vec![
        required(f("name", "Name", FieldType::Text, "name", "IconHistory")),
        required(select("status", "Status", "status", "IconCircleDot", &[("NOT_STARTED", "Not started", "gray"), ("RUNNING", "Running", "blue"), ("COMPLETED", "Completed", "green"), ("FAILED", "Failed", "red"), ("WAITING", "Waiting", "yellow")])),
        to_one("workflow", "Workflow", "workflow", "workflow", "runs", "IconSettingsAutomation"),
        f("startedAt", "Started at", FieldType::DateTime, "started_at", "IconCalendarEvent"),
        f("endedAt", "Ended at", FieldType::DateTime, "ended_at", "IconCalendarEvent"),
    ];
    run.extend(common());
    objects.push(ObjectDecl {
        name_singular: "workflowRun",
        name_plural: "workflowRuns",
        label_singular: "Workflow run",
        label_plural: "Workflow runs",
        icon: "IconHistory",
        table: "workflow_run",
        system: false,
        label_identifier: "name",
        image_identifier: None,
        fields: run,
    });

    objects.push(ObjectDecl {
        name_singular: "attachment",
        name_plural: "attachments",
        label_singular: "Attachment",
        label_plural: "Attachments",
        icon: "IconFileImport",
        table: "attachment",
        system: true,
        label_identifier: "name",
        image_identifier: None,
        fields: vec![
            required(f("name", "Name", FieldType::Text, "name", "IconFileUpload")),
            f("fullPath", "Full path", FieldType::Text, "full_path", "IconLink"),
            f("type", "Type", FieldType::Text, "type", "IconList"),
            to_one("author", "Author", "author", "workspaceMember", "authoredAttachments", "IconCircleUser"),
            to_one("person", "Person", "person", "person", "attachments", "IconUser"),
            to_one("company", "Company", "company", "company", "attachments", "IconBuildingSkyscraper"),
            to_one("opportunity", "Opportunity", "opportunity", "opportunity", "attachments", "IconTargetArrow"),
            to_one("note", "Note", "note", "note", "attachments", "IconNotes"),
            to_one("task", "Task", "task", "task", "attachments", "IconCheckbox"),
        ],
    });

    objects.push(ObjectDecl {
        name_singular: "favorite",
        name_plural: "favorites",
        label_singular: "Favorite",
        label_plural: "Favorites",
        icon: "IconHeart",
        table: "favorite",
        system: true,
        label_identifier: "position",
        image_identifier: None,
        fields: vec![
            sys(f("position", "Position", FieldType::Position, "position", "IconList")),
            to_one("workspaceMember", "Workspace member", "workspace_member", "workspaceMember", "favorites", "IconCircleUser"),
            to_one("person", "Person", "person", "person", "favorites", "IconUser"),
            to_one("company", "Company", "company", "company", "favorites", "IconBuildingSkyscraper"),
            to_one("opportunity", "Opportunity", "opportunity", "opportunity", "favorites", "IconTargetArrow"),
            to_one("note", "Note", "note", "note", "favorites", "IconNotes"),
            to_one("task", "Task", "task", "task", "favorites", "IconCheckbox"),
            to_one("dashboard", "Dashboard", "dashboard", "dashboard", "favorites", "IconLayoutDashboard"),
        ],
    });

    objects.push(ObjectDecl {
        name_singular: "timelineActivity",
        name_plural: "timelineActivities",
        label_singular: "Timeline activity",
        label_plural: "Timeline activities",
        icon: "IconTimelineEvent",
        table: "timeline_activity",
        system: true,
        label_identifier: "name",
        image_identifier: None,
        fields: vec![
            f("happensAt", "Creation date", FieldType::DateTime, "happens_at", "IconCalendar"),
            required(f("name", "Event name", FieldType::Text, "name", "IconAbc")),
            f("properties", "Event details", FieldType::Json, "properties_json", "IconListDetails"),
            f("linkedRecordCachedName", "Linked record cached name", FieldType::Text, "linked_record_cached_name", "IconAbc"),
            f("linkedRecordId", "Linked record id", FieldType::Uuid, "linked_record_id", "IconAbc"),
            f("linkedObjectMetadataId", "Linked object metadata id", FieldType::Uuid, "linked_object_metadata_id", "IconAbc"),
            to_one("workspaceMember", "Workspace member", "workspace_member", "workspaceMember", "timelineActivities", "IconCircleUser"),
            to_one("person", "Person", "person", "person", "timelineActivities", "IconUser"),
            to_one("company", "Company", "company", "company", "timelineActivities", "IconBuildingSkyscraper"),
            to_one("opportunity", "Opportunity", "opportunity", "opportunity", "timelineActivities", "IconTargetArrow"),
            to_one("note", "Note", "note", "note", "timelineActivities", "IconNotes"),
            to_one("task", "Task", "task", "task", "timelineActivities", "IconCheckbox"),
        ],
    });

    objects
}

/// Writes the standard objects into the metadata tables where they are missing. Rows that exist
/// are left as the owner has them.
pub async fn sync(pool: &SqlitePool) -> Result<()> {
    for (i, object) in standard_objects().into_iter().enumerate() {
        let object_id: Option<(String,)> = sqlx::query_as("SELECT id FROM object_meta WHERE name_singular = ?")
            .bind(object.name_singular)
            .fetch_optional(pool)
            .await?;
        let object_id = match object_id {
            Some((id,)) => id,
            None => {
                let id = uuid::Uuid::now_v7().to_string();
                sqlx::query(
                    "INSERT INTO object_meta (id, name_singular, name_plural, label_singular, label_plural, icon, \
                     table_name, is_custom, is_active, is_system, position) VALUES (?, ?, ?, ?, ?, ?, ?, 0, 1, ?, ?)",
                )
                .bind(&id)
                .bind(object.name_singular)
                .bind(object.name_plural)
                .bind(object.label_singular)
                .bind(object.label_plural)
                .bind(object.icon)
                .bind(object.table)
                .bind(object.system)
                .bind(i as f64)
                .execute(pool)
                .await?;
                id
            }
        };
        for (j, field) in object.fields.iter().enumerate() {
            let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM field_meta WHERE object_id = ? AND name = ?")
                .bind(&object_id)
                .bind(field.name)
                .fetch_optional(pool)
                .await?;
            if exists.is_some() {
                continue;
            }
            sqlx::query(
                "INSERT INTO field_meta (id, object_id, name, label, type, column_name, options_json, is_custom, is_active, \
                 is_nullable, is_unique, is_system, position, description, icon) VALUES (?, ?, ?, ?, ?, ?, ?, 0, 1, ?, ?, ?, ?, ?, ?)",
            )
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(&object_id)
            .bind(field.name)
            .bind(field.label)
            .bind(field.r#type.as_str())
            .bind(field.column)
            .bind(serde_json::to_string(&field.options)?)
            .bind(field.nullable)
            .bind(field.unique)
            .bind(field.system)
            .bind(j as f64)
            .bind(field.description)
            .bind(field.icon)
            .execute(pool)
            .await?;
        }
        for (column, field_name) in [("label_identifier_field_id", Some(object.label_identifier)), ("image_identifier_field_id", object.image_identifier)] {
            let Some(field_name) = field_name else { continue };
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE object_meta SET {column} = (SELECT id FROM field_meta WHERE object_id = ? AND name = ?) \
                 WHERE id = ? AND {column} IS NULL"
            )))
            .bind(&object_id)
            .bind(field_name)
            .bind(&object_id)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}
