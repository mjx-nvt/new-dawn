# Reference Projects Audit — Ecomobi / NetNam / VinaCapital MKT Infrastructure

_Audited 2026-10-10 from `../Ref/` (the three old Next.js projects next to this repo). Factual findings only; the New Dawn design lives in `plan.md`._

## What they are

Three forks of one codebase ("MKT Infrastructure" — an internal marketing-intelligence tool with a knowledge base, research library, AI companions and planning stubs).

| | eco_mkt_infra | netnam_mkt_infra | vinacapital_mkt_infra |
|---|---|---|---|
| Lineage | Origin (May 2026, 31 commits) | Exact clone of eco (same 31 commits) + theme swap | Fork, diverged (39 commits, to 30 Jun 2026) |
| Font | DM Sans | Plus Jakarta Sans | Helvetica Now + Georgia (serif) |
| Brand colour | `#F43C34` red | `#35B033` green | `#D71920` red + navy `#333B52`, off-white canvas `#EDECE8` |
| Nav differences | Full set (Services, Creators, Client Insight, Creator Insight, Current Projects, Team Performance) | Creators + Creator Insight removed from nav (routes still exist) | Client Insight/Creators/Services replaced by **Investor Insight** (Institutional / Retail); Current Projects + Team Performance removed |
| Auth | NextAuth v5 credentials + Prisma | same | **Stubbed out** — cloned DB/auth env broke on Vercel, so `lib/auth.ts` returns a fixed admin session; app is public |
| Prisma schema | identical | identical | identical |

## Shared stack
Next.js 16 App Router, TypeScript strict, plain CSS modules (no Tailwind), Prisma 7 + Neon Postgres, NextAuth v5 (credentials, roles ADMIN/EDITOR/VIEWER), Anthropic SDK (models centralised in `lib/ai-models.ts`), Vercel Blob for PDFs, AES-256-GCM encrypted API key in `AppSettings` singleton, Vercel hosting.

## Where data lives
- **DB (Prisma)** holds only dynamic/operational data: `User`, `Session`, `AppSettings`, `Suggestion` (Vision Companion log), `ResearchSource`, `Insight` (with `applicabilityPortfolios/Brands/Icps/Personas` string[] + GIN indexes), `ResearchQuery`, `EntityDraft` (human-in-the-loop AI drafting for ICP/persona).
- **Static TypeScript** holds all domain knowledge: `data/portfolio/accounts.ts` (1,750 lines; Portfolio → Brand → Project hierarchy), `data/services/` (modules, service lines, deal USPs), `data/icps/`, `data/personas/`, `data/creators/` (archetypes, markets, network). VinaCapital puts investor data in route-colocated `_data/` folders instead.
- Three-layer SSOT rule: data layer → helper layer (`lib/*/helpers.ts`, `data/*/helpers.ts`) → render layer. Components never import data files or Prisma directly.
- Cross-links are typed via literal union slugs (`ModuleSlug`, `ServiceLineSlug`) — a typo fails the build.

## Shared UI/system patterns worth carrying forward
- **Transparency / ghost pattern** (D-008): every dossier section always renders; empty fields show "Not yet captured", empty lists show 2–3 dashed ghost cards. Type-conditional sections are omitted, not ghosted (D-024).
- Dossier pages = numbered sections in lettered groups (portfolio 14 sections / 7 groups; brand 11 / 5; archetype 10 / 5; market 7 / 4).
- URL-state tabs + filters (`?tab=`, `?role=`…) via `router.push`, never local state (D-022, D-015).
- AI drawer pattern: right-side 480px slide-in, ESC/backdrop closes, floating bottom-right trigger (`VisionCompanion.tsx` canonical).
- Helpers accept `string` URL params and cast internally, return `undefined` → `notFound()` (D-023).
- Slug on every entity, auto-generated from name, stable.
- Theme fully tokenised in `globals.css` (`--red`, `--font-sans`, `--canvas`, brand ramp 50–950, neutral ramp). VinaCapital note: "`--red` token name is historical" — tokens are literal, not semantic.

## Dev workflow tooling (identical across repos)
`CLAUDE.md` (source of truth), `DECISIONS.md` (D-001…D-028), `LAST_BUILD.md`, `CONSOLIDATION_REPORT.md`, `.claude/agents/architect-reviewer.md`, `.claude/agents/migration-safety.md`, `.claude/commands/deploy-check.md`, `.claude/skills/<brand>-ui-patterns/SKILL.md`. THINK → ALIGN → SEE → BUILD workflow; surgical changes preferred.

## Pain points observed (the case for New Dawn)
1. Clone-and-diverge: fixes don't propagate (e.g. CONSOLIDATION_REPORT C-001 auth gap exists in all three copies).
2. Cloned env/DB coupling broke VinaCapital auth → stubbed → public deployment.
3. Module identity is baked into folder names and URLs (`client-insight`, `investor-insight`), the `navTree` constant, and per-page JSX section lists — changing a tenant's module set means forking pages.
4. Content changes are code deploys (static TS).
5. Research `applicability*` fields are hard-wired to four entity kinds; new domain entities (investors, creators) can't be cross-linked without schema changes.
6. Brand theming requires editing globals.css + CLAUDE.md + a per-brand skill; status colours (green/navy) are not semantic.

## What New Dawn keeps from them
The ghost pattern, dossier section rhythm, URL-state filters, the drawer behaviour, the helper-layer discipline, the `.claude/` tooling style, the DECISIONS log — and the content itself (Phase 4 imports the Ecomobi `data/*.ts` into an archive workspace).
