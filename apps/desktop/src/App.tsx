import { startTransition, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle, ArrowDown, ArrowUp, BarChart3, Bot, CalendarDays, Check, CheckCircle2, ChevronDown,
  ChevronRight, CircleDollarSign, Clock3, Coins, Gauge, Home as HomeIcon, Languages,
  ListChecks, Moon, RefreshCw, Repeat2, Search, Settings, ShieldCheck, Sparkles, Sun, Waypoints, X,
} from "lucide-react";
import logoUrl from "../../../assets/logo-icon.png";
import { api } from "./api";
import {
  dateTime, DEFAULT_LANGUAGE, duration, evidenceLabel, findingCopy, locale, money, outcomeLabel,
  reasonLabel, useCopy, type Copy, type Language,
} from "./i18n";
import type { CompareData, Finding, HomeData, SessionDetail, SessionSummary, SourceState, View } from "./types";

function sourceTone(source: string) { if (source.includes("Claude")) return "purple"; if (source.includes("Gemini")) return "blue"; if (source.includes("Hermes")) return "orange"; return "green"; }

export function App() {
  const [view, setView] = useState<View>(() => {
    const requested = new URLSearchParams(location.search).get("view") as View | null;
    return requested && ["home", "sessions", "discover", "compare", "settings"].includes(requested) ? requested : "home";
  });
  const [language, setLanguage] = useState<Language>(() => {
    const requested = new URLSearchParams(location.search).get("lang") as Language | null;
    return requested === "zh" || requested === "en" ? requested : (localStorage.getItem("language") as Language) || DEFAULT_LANGUAGE;
  });
  const [dark, setDark] = useState(() => localStorage.getItem("theme") ? localStorage.getItem("theme") === "dark" : matchMedia("(prefers-color-scheme: dark)").matches);
  const [sources, setSources] = useState<SourceState | null>(null);
  const [home, setHome] = useState<HomeData | null>(null);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [findings, setFindings] = useState<Finding[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [detail, setDetail] = useState<SessionDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const t = useCopy(language);

  async function loadAll(refresh = false) {
    setLoading(true); setError("");
    try {
      if (refresh) await api.refreshSessions();
      const [sourceData, homeData, sessionData, findingData] = await Promise.all([api.detectSources(), api.loadHome(), api.listSessions(), api.listFindings()]);
      setSources(sourceData); setHome(homeData); setSessions(sessionData.sessions); setFindings(findingData);
      setSelectedId((current) => sessionData.sessions.some(({id}) => id === current) ? current : sessionData.sessions[0]?.id || "");
    } catch (reason) { setError(String(reason)); } finally { setLoading(false); }
  }

  useEffect(() => { void loadAll(); }, []);
  useEffect(() => { document.documentElement.dataset.theme = dark ? "dark" : "light"; localStorage.setItem("theme", dark ? "dark" : "light"); }, [dark]);
  useEffect(() => { document.documentElement.lang = language === "zh" ? "zh-CN" : "en"; localStorage.setItem("language", language); }, [language]);
  useEffect(() => { let active = true; setDetail(null); if (selectedId) void api.getSession(selectedId).then((value) => { if (active) setDetail(value); }).catch(() => { if (active) setDetail(null); }); return () => { active = false; }; }, [selectedId, sessions]);
  const navigate = (next: View) => startTransition(() => setView(next));

  if (loading) return <Loading t={t} />;
  if (error) return <ErrorState message={error} retry={loadAll} t={t} />;
  if (!sources || sources.sessionCount === 0) return <Welcome t={t} onStart={loadAll} />;
  return <div className="app-shell">
    <Sidebar view={view} navigate={navigate} healthy={home?.dataHealth.confidence !== "low"} t={t} />
    <main className="workspace">
      {view === "home" && home && <Home sessions={sessions} findings={findings} language={language} t={t} openSessions={() => navigate("sessions")} openDiscover={() => navigate("discover")} refresh={() => loadAll(true)} />}
      {view === "sessions" && <Sessions sessions={sessions} selectedId={selectedId} select={setSelectedId} detail={detail} language={language} t={t} refresh={() => loadAll(true)} />}
      {view === "discover" && <Discover findings={findings} language={language} t={t} />}
      {view === "compare" && <Compare sessions={sessions} initialId={selectedId} language={language} t={t} />}
      {view === "settings" && <SettingsView sources={sources} language={language} setLanguage={setLanguage} dark={dark} setDark={setDark} t={t} rescan={() => loadAll(true)} />}
    </main>
  </div>;
}

function Sidebar({ view, navigate, healthy, t }: { view: View; navigate: (view: View) => void; healthy: boolean; t: Copy }) {
  const items = [["home", HomeIcon, t.home], ["sessions", ListChecks, t.sessions], ["discover", Search, t.discover], ["compare", Waypoints, t.compare]] as const;
  return <aside className="sidebar">
    <div className="window-drag"><span /></div>
    <div className="brand"><img src={logoUrl} alt="" /><strong>AgentTrace</strong></div>
    <nav aria-label="Primary">{items.map(([id, Icon, label]) => <button key={id} className={view === id ? "active" : ""} onClick={() => navigate(id)}><Icon /><span>{label}</span></button>)}</nav>
    <button className={`settings-link ${view === "settings" ? "active" : ""}`} onClick={() => navigate("settings")}><Settings />{t.settings}</button>
    <div className="local-state"><span className={`status-dot ${healthy ? "" : "off"}`} />{healthy ? t.systemHealthy : t.dataNeedsAttention}</div>
  </aside>;
}

function PageHeader({ title, children }: { title: string; children?: React.ReactNode }) { return <header className="page-header"><h1>{title}</h1>{children}</header>; }

export function sessionsInRange(sessions: SessionSummary[], hours: number, now = Date.now()) {
  const since = now - hours * 60 * 60 * 1000;
  return sessions.filter((session) => {
    const startedAt = Date.parse(session.startedAt);
    return Number.isFinite(startedAt) && startedAt >= since && startedAt <= now;
  });
}

export function healthTrend(sessions: SessionSummary[]) {
  const ordered = [...sessions].sort((a,b) => Date.parse(a.startedAt) - Date.parse(b.startedAt));
  return ordered.map((session,index) => `${index ? "L" : "M"}${ordered.length === 1 ? 400 : index * 800 / (ordered.length - 1)} ${240 - session.health * 2.4}`).join(" ");
}

export function compactTokens(value: number) {
  const [divisor,suffix] = Math.abs(value) >= 1e12 ? [1e12,"T"] : Math.abs(value) >= 1e6 ? [1e6,"M"] : Math.abs(value) >= 1e3 ? [1e3,"K"] : [1,""];
  return suffix ? `${(value / divisor).toFixed(1)}${suffix}` : Math.round(value).toString();
}

function Home({ sessions, findings, language, t, openSessions, openDiscover, refresh }: { sessions: SessionSummary[]; findings: Finding[]; language: Language; t: Copy; openSessions: () => void; openDiscover: () => void; refresh: () => void }) {
  const [rangeHours, setRangeHours] = useState(24);
  const visible = useMemo(() => sessionsInRange(sessions, rangeHours), [sessions, rangeHours]);
  const smoothSessions = visible.filter((session) => session.status === "smooth").length;
  const averageHealth = visible.length ? visible.reduce((sum, session) => sum + session.health, 0) / visible.length : 0;
  const totalCost = visible.reduce((sum, session) => sum + session.cost, 0);
  const attention = findings.filter((finding) => visible.some((session) => session.id === finding.sessionId)).slice(0, 3);
  const attentionRate = visible.length ? Math.round((visible.length - smoothSessions) / visible.length * 1000) / 10 : 0;
  const p95 = [...visible].sort((a,b)=>a.durationSec-b.durationSec)[Math.max(0, Math.ceil(visible.length*.95)-1)]?.durationSec || 0;
  const trend = healthTrend(visible);
  const metrics = [
    [t.totalSessions, visible.length.toLocaleString(locale(language)), ListChecks, "blue"], [t.healthScore, `${Math.round(averageHealth)}%`, Gauge, "green"],
    [t.issueRate, `${attentionRate}%`, AlertTriangle, "orange"], [t.p95Latency, duration(p95, language), Clock3, "purple"], [t.spend, money(totalCost, language), CircleDollarSign, "blue"],
  ] as const;
  return <div className="page home-page">
    <PageHeader title={t.home}><div className="range-controls">{[[24, `24 ${language === "zh" ? "小时" : "hours"}`], [168, `7 ${language === "zh" ? "天" : "days"}`], [720, `30 ${language === "zh" ? "天" : "days"}`]] .map(([hours, label]) => <button key={hours} className={rangeHours === hours ? "active" : ""} aria-pressed={rangeHours === hours} onClick={() => setRangeHours(Number(hours))}>{label}</button>)}<button aria-label={t.refresh} onClick={refresh}><RefreshCw /></button></div></PageHeader>
    <section className="metric-grid">{metrics.map(([label,value,Icon,tone]) => <article className="metric-card" key={label}><div><small>{label}</small><strong>{value}</strong></div><span className={`metric-icon ${tone}`}><Icon /></span></article>)}</section>
    <section className="home-grid">
      <article className="chart-card panel"><div className="section-heading"><h2>{t.healthTrend}</h2><span>{t.healthScore}</span></div><div className="trend-chart" aria-label={t.healthTrend}><svg viewBox="0 0 800 240" preserveAspectRatio="none"><path className="chart-line" d={trend}/></svg><span className="axis top">100%</span><span className="axis bottom">0%</span></div><div className="chart-legend"><span><i className="green-dot" />{t.healthySessions} {Math.round(averageHealth)}%</span><span><i className="orange-dot" />{t.needsAttention} {attentionRate}%</span></div></article>
      <article className="anomaly-card panel"><div className="section-heading"><h2>{t.attention} <b>{attention.length}</b></h2><button onClick={openDiscover}>{t.viewAll}<ChevronRight /></button></div>{attention.map((item) => { const copy = findingCopy(item, language); return <button className="anomaly-row" onClick={openDiscover} key={`${item.sessionId}-${item.kind}`}><span className={`severity-dot ${item.severity}`} /><span><strong>{copy.title}</strong><small>{copy.impact}</small></span></button>; })}</article>
    </section>
    <section className="recent-table panel"><div className="section-heading"><h2>{t.recent}</h2><button onClick={openSessions}>{t.viewAll}<ChevronRight /></button></div>{visible.slice(0,5).map(session => <SessionTableRow key={session.id} session={session} language={language} t={t} onClick={openSessions} />)}</section>
  </div>;
}

function Sessions({ sessions, selectedId, select, detail, language, t, refresh }: { sessions: SessionSummary[]; selectedId: string; select: (id:string)=>void; detail: SessionDetail|null; language: Language; t: Copy; refresh:()=>void }) {
  const [query,setQuery]=useState(""); const [filter,setFilter]=useState("all"); const [modal,setModal]=useState(false);
  const visible=useMemo(()=>sessions.filter(s=>(filter==="all"||s.status===filter)&&`${s.name} ${s.project} ${s.source}`.toLowerCase().includes(query.trim().toLowerCase())),[sessions,query,filter]);
  const open=(id:string)=>{ select(id); setModal(true); };
  return <div className="page sessions-page"><PageHeader title={t.sessions}/><div className="sessions-toolbar"><label className="search-box"><Search/><input value={query} onChange={e=>setQuery(e.target.value)} placeholder={t.search}/></label><div className="filter-buttons">{[["all",t.all],["smooth",t.smooth],["attention",t.needsAttention]].map(([id,label])=><button key={id} className={filter===id?`active ${id}`:""} onClick={()=>setFilter(id)}>{label}</button>)}<button aria-label={t.refresh} onClick={refresh}><RefreshCw/></button></div></div>
    <section className="session-list">{visible.map(session=><SessionTableRow key={session.id} session={session} language={language} t={t} onClick={()=>open(session.id)} selected={selectedId===session.id}/>)}{!visible.length&&<EmptyState text={t.noSessions}/>}</section><footer className="list-count">{t.sessionCount(visible.length)}</footer>
    {modal&&<SessionModal detail={detail?.session.id === selectedId ? detail : null} language={language} t={t} close={()=>setModal(false)}/>}</div>;
}

function SessionTableRow({ session, language, t, onClick, selected=false }: { session:SessionSummary; language:Language; t:Copy; onClick:()=>void; selected?:boolean }) { return <button className={`session-table-row ${selected?"selected":""}`} onClick={onClick}><span className={`session-icon ${sourceTone(session.source)}`}><Bot/></span><span className="session-title"><strong>{session.name}</strong><small>{session.project} · {session.source}</small></span><span className={`status-pill ${session.status}`}>{session.status==="smooth"?t.smooth:t.needsAttention}</span><span className="row-meta"><Clock3/>{duration(session.durationSec,language)}</span><span className="row-meta"><CalendarDays/>{dateTime(session.startedAt,language)}</span><ChevronRight/></button>; }

function SessionModal({ detail, language, t, close }: { detail:SessionDetail|null; language:Language; t:Copy; close:()=>void }) { if(!detail)return null; const s=detail.session; return <div className="modal-backdrop" onMouseDown={close}><article className="session-modal" onMouseDown={e=>e.stopPropagation()}><button className="modal-close" onClick={close}><X/></button><div className="modal-title"><span className={`session-icon ${sourceTone(s.source)}`}><Bot/></span><div><h2>{s.name}</h2><p>{s.project} · {s.source} · {s.model}</p></div></div><div className="detail-metrics"><div><small>{t.health}</small><strong>{s.health}</strong></div><div><small>{t.duration}</small><strong>{duration(s.durationSec,language)}</strong></div><div><small>{t.spend}</small><strong>{money(s.cost,language)}</strong></div><div><small>{t.tokens}</small><strong>{compactTokens(s.tokens)}</strong></div></div>{detail.findings.length>0&&<div className="modal-findings"><h3>{t.attention}</h3>{detail.findings.map(f=>{const c=findingCopy(f,language);return <div key={f.id}><AlertTriangle/><span><strong>{c.title}</strong><small>{c.impact}</small></span></div>})}</div>}</article></div>; }

function Discover({ findings, language, t }: { findings:Finding[]; language:Language; t:Copy }) { const [expanded,setExpanded]=useState<string>(findings[0]?.id||""); return <div className="page discover-page"><PageHeader title={t.discover}/><section className="discover-intro panel"><span><Check/></span><div><h2>{t.found(findings.length)}</h2><p>{t.foundBody}</p></div></section><section className="finding-list">{findings.map((finding,index)=>{const c=findingCopy(finding,language),open=expanded===finding.id;return <article className="finding-card" key={finding.id}><button className="finding-summary" onClick={()=>setExpanded(open?"":finding.id)}><span className={`finding-icon ${finding.severity}`}>{finding.kind==="loop"?<Repeat2/>:finding.kind==="latency"?<Clock3/>:<AlertTriangle/>}</span><span className="finding-copy"><b>{index+1}</b><strong>{c.title}</strong><small>{c.impact}</small></span><ChevronDown className={open?"rotate":""}/></button><div className={`finding-actions ${open?"open":""}`}><span>{t.evidence}（{finding.evidence.length}）</span><div><button onClick={()=>setExpanded(finding.id)}>{t.evidence}</button><button className="primary" onClick={()=>setExpanded(finding.id)}>{t.improve}</button></div></div>{open&&<div className="finding-detail"><div><h4>{t.improve}</h4><p>{c.recommendation}</p></div><div><h4>{t.evidence}</h4>{finding.evidence.map(e=><p key={e.kind}><strong>{evidenceLabel(e.kind,language)}：</strong>{e.value}</p>)}</div></div>}</article>})}</section><section className="positive-banner"><CheckCircle2/><div><strong>{t.doingWell}</strong><p>{t.doingWellBody}</p></div><ChevronRight/></section></div>; }

function Compare({ sessions, initialId, language, t }: { sessions:SessionSummary[]; initialId:string; language:Language; t:Copy }) { const [currentId,setCurrentId]=useState(initialId||sessions[0]?.id||""); const [previousId,setPreviousId]=useState(sessions.find(s=>s.id!==currentId)?.id||""); const [data,setData]=useState<CompareData|null>(null); useEffect(()=>{ if(currentId&&previousId&&currentId!==previousId) void api.compareSessions(currentId,previousId).then(setData).catch(()=>setData(null)); },[currentId,previousId]); const metricLabel=(kind:string)=>({duration:t.duration,cost:t.cost,tool_failures:t.toolFailures,repeated_work:t.repeatedWork}[kind]||kind); return <div className="page compare-page"><PageHeader title={t.compareTitle}/><section className="compare-selectors"><SessionSelect label={t.current} tone="green" value={currentId} setValue={setCurrentId} sessions={sessions}/><span>{t.versus}</span><SessionSelect label={t.previous} tone="blue" value={previousId} setValue={setPreviousId} sessions={sessions}/></section>{data?<><section className="compare-hero"><span><Check/></span><h2>{outcomeLabel(data.outcome,language)}</h2></section><section className="comparison panel">{data.metrics.map(metric=>{const improved=metric.lowerIsBetter?metric.current<=metric.previous:metric.current>=metric.previous; const Direction=metric.current<=metric.previous?ArrowDown:ArrowUp; return <div className="compare-row" key={metric.kind}><span className="metric-name"><i><MetricIcon kind={metric.kind}/></i>{metricLabel(metric.kind)}</span><strong className={improved?"good":""}>{formatMetric(metric.current,metric.unit,language)}</strong><span>vs</span><strong>{formatMetric(metric.previous,metric.unit,language)}</strong><em className={improved?"good":"bad"}><Direction/>{delta(metric.current,metric.previous,metric.unit,language)}</em></div>})}</section><section className="why panel"><h3>{t.why}</h3>{data.reasons.map(reason=><p key={reason}><CheckCircle2/>{reasonLabel(reason,language)}</p>)}</section><details className="technical"><summary>{t.technical}</summary><p>{t.health}: {data.current.health} vs {data.previous.health} · {t.tokens}: {compactTokens(data.current.tokens)} vs {compactTokens(data.previous.tokens)}</p></details></>:<EmptyState text={t.noComparison}/>}</div>; }

function SessionSelect({label,tone,value,setValue,sessions}:{label:string;tone:string;value:string;setValue:(v:string)=>void;sessions:SessionSummary[]}) { return <label className={`session-selector ${tone}`}><span>{label}</span><div><Sparkles/><select value={value} onChange={e=>setValue(e.target.value)}>{sessions.map(s=><option key={s.id} value={s.id}>{s.name}</option>)}</select><ChevronDown/></div></label>; }
function MetricIcon({kind}:{kind:string}) { if(kind==="duration")return <Clock3/>; if(kind==="cost")return <Coins/>; if(kind==="tool_failures")return <AlertTriangle/>; return <Repeat2/>; }

function SettingsView({sources,language,setLanguage,dark,setDark,t,rescan}:{sources:SourceState;language:Language;setLanguage:(v:Language)=>void;dark:boolean;setDark:(v:boolean)=>void;t:Copy;rescan:()=>void}) { return <div className="page settings-page"><PageHeader title={t.settings}/><section className="settings-group"><h2>{t.localSources}</h2>{sources.sources.map(source=><div className="setting-row" key={source.name}><span className="session-icon blue"><Bot/></span><div><strong>{source.name}</strong><small>{source.detected?t.detected:t.notFound}</small></div><span className={`status-dot ${source.detected?"":"off"}`}/></div>)}<button className="secondary" onClick={rescan}><RefreshCw/>{t.rescan}</button></section><section className="settings-group"><h2>{t.appearance}</h2><div className="setting-row"><span className="session-icon purple">{dark?<Moon/>:<Sun/>}</span><div><strong>{dark?t.dark:t.light}</strong><small>{t.followPreference}</small></div><button className="toggle" aria-pressed={dark} onClick={()=>setDark(!dark)}><span/></button></div><div className="setting-row"><span className="session-icon green"><Languages/></span><div><strong>{t.language}</strong><small>简体中文 / English</small></div><select value={language} onChange={e=>setLanguage(e.target.value as Language)}><option value="zh">简体中文</option><option value="en">English</option></select></div></section><section className="privacy-note"><ShieldCheck/><div><strong>{t.privacy}</strong><p>{t.localOnlyBody}</p></div></section></div>; }

function Welcome({t,onStart}:{t:Copy;onStart:()=>void}) { const cards=[[ListChecks,t.tracking,t.trackingBody,"blue"],[Gauge,t.insights,t.insightsBody,"green"],[Sparkles,t.opportunities,t.opportunitiesBody,"purple"],[BarChart3,t.optimize,t.optimizeBody,"orange"]] as const; return <div className="welcome-shell"><aside className="welcome-side"><div className="window-drag"><span/></div><div className="brand"><img src={logoUrl} alt=""/><strong>AgentTrace</strong></div><nav><button className="active"><HomeIcon/>{t.home}</button><button><ListChecks/>{t.sessions}</button><button><Search/>{t.discover}</button><button><BarChart3/>{t.compare}</button></nav><div className="local-state"><span className="status-dot"/>{t.systemHealthy}</div></aside><main className="welcome"><img className="welcome-logo" src={logoUrl} alt=""/><h1>{t.welcome}</h1><p>{t.welcomeBody}</p><div className="welcome-cards">{cards.map(([Icon,title,body,tone])=><article key={title}><span className={`session-icon ${tone}`}><Icon/></span><h2>{title}</h2><p>{body}</p></article>)}</div><button className="welcome-start" onClick={onStart}>{t.start}</button><small>{t.privacy}</small></main></div>; }

function EmptyState({text}:{text:string}) { return <div className="empty-state"><Search/><p>{text}</p></div>; }
function Loading({t}:{t:Copy}) { return <div className="loading"><img src={logoUrl} alt=""/><p>{t.scanning}</p></div>; }
function ErrorState({message,retry,t}:{message:string;retry:()=>void;t:Copy}) { return <div className="loading error"><AlertTriangle/><h1>{t.loadFailed}</h1><p>{message}</p><button className="primary" onClick={retry}><RefreshCw/>{t.retry}</button></div>; }
function formatMetric(value:number,unit:string,language:Language) { if(unit==="usd")return money(value,language); if(unit==="sec")return duration(value,language); return Math.round(value).toLocaleString(locale(language)); }
function delta(current:number,previous:number,unit:string,language:Language) { const value=Math.abs(current-previous); const formatted=formatMetric(value,unit,language); return current<=previous?`${useCopy(language).saved} ${formatted}`:`+${formatted}`; }
