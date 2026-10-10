(() => {
  "use strict";

  const storageKey = "itanongmo.language";
  const filipino = Object.freeze({
    "AI response draft": "AI draft ng sagot",
    "Verified facts": "Na-verify na impormasyon",
    "Refresh drafts": "I-refresh ang mga draft",
    "Completed drafts": "Mga natapos na draft",
    "Review and edit response": "Suriin at i-edit ang sagot",
    "Sources and limitations": "Mga source at limitasyon",
    "Publish reviewed response": "I-publish ang nasuring sagot",
    "AI controls could not load. You can still reply manually.": "Hindi ma-load ang AI controls. Maaari pa ring mag-reply nang manu-mano.",
    "Ask using AI": "Magtanong gamit ang AI",
    "Describe the records you need. AI can prepare a draft for you to review. Enter your request title below first.": "Ilarawan ang mga rekord na kailangan mo. Makagagawa ang AI ng draft na susuriin mo. Ilagay muna ang pamagat ng kahilingan sa ibaba.",
    "What would you like to ask?": "Ano ang gusto mong itanong?",
    "Generate AI draft": "Gumawa ng AI draft",
    "Apply draft": "Gamitin ang draft",
    "Check the suggested agencies against the agency list. AI may make mistakes; you decide what to submit.": "Suriin ang mga mungkahing ahensiya gamit ang listahan. Maaaring magkamali ang AI; ikaw ang magpapasya kung ano ang isusumite.",
    "Preparing your AI draft…": "Ginagawa ang iyong AI draft…",
    "Enter the manual request title first.": "Ilagay muna ang pamagat ng kahilingan.",
    "Describe the records you need in 20–8,000 characters.": "Ilarawan ang kailangan mong mga rekord sa 20–8,000 character.",
    "Your inputs changed. Generate a new draft.": "Nagbago ang iyong sagot. Gumawa ng bagong draft.",
    "Review the draft and suggested agencies before applying it.": "Suriin ang draft at mga mungkahing ahensiya bago ito gamitin.",
    "Draft applied. Check your agency and details, then submit when ready.": "Nailagay na ang draft. Suriin ang ahensiya at mga detalye, saka isumite kapag handa na.",
    "AI returned an invalid JSON draft.": "Hindi valid ang JSON draft mula sa AI.",
    "AI returned invalid form values.": "Hindi valid ang form values mula sa AI.",
    "AI returned invalid agency suggestions.": "Hindi valid ang mga mungkahing ahensiya mula sa AI.",
    "AI service is temporarily unavailable. You can continue manually.": "Pansamantalang hindi available ang AI. Maaari kang magpatuloy nang manu-mano.",
    "AI usage limit reached. Try again later or continue manually.": "Naabot ang AI usage limit. Subukan muli mamaya o magpatuloy nang manu-mano.",

    "Main navigation": "Pangunahing nabigasyon",
    "Footer navigation": "Nabigasyon sa ibaba",
    "Language": "Wika",
    "Track": "Subaybayan",
    "Portal capabilities": "Mga gamit ng portal",
    "Page not found — iTanongMo": "Hindi makita ang pahina — iTanongMo",
    "About — iTanongMo": "Tungkol dito — iTanongMo",
    "Accessibility — iTanongMo": "Aksesibilidad — iTanongMo",
    "Citizen portal — iTanongMo": "Portal ng mamamayan — iTanongMo",
    "Government portal — iTanongMo": "Portal ng pamahalaan — iTanongMo",
    "Government workspace — iTanongMo": "Workspace ng pamahalaan — iTanongMo",
    "iTanongMo — AppBuildersPH Hackathon": "iTanongMo — AppBuildersPH Hackathon",
    "Sign in — iTanongMo": "Mag-sign in — iTanongMo",
    "Privacy — iTanongMo": "Pagkapribado — iTanongMo",
    "Make an FOI request — iTanongMo": "Maghain ng FOI request — iTanongMo",
    "Track an FOI request — iTanongMo": "Subaybayan ang FOI request — iTanongMo",
    "Make a request": "Maghain ng kahilingan",
    "Track a request": "Subaybayan ang kahilingan",
    "About": "Tungkol dito",
    "Portal sign in": "Mag-sign in sa portal",
    "Citizen portal": "Portal ng mamamayan",
    "Government portal": "Portal ng pamahalaan",
    "Sign in": "Mag-sign in",
    "Privacy": "Pagkapribado",
    "Accessibility": "Aksesibilidad",
    "Available after the public Rust API is configured.": "Magagamit kapag na-configure na ang pampublikong Rust API.",
    "Page not found": "Hindi makita ang pahina",
    "Return to iTanongMo": "Bumalik sa iTanongMo",
    "AppBuildersPH Hackathon edition": "Edisyon para sa AppBuildersPH Hackathon",
    "Public information, easier to request and manage": "Pampublikong impormasyon, mas madaling hilingin at asikasuhin",
    "Send one FOI submission to one or more government agencies. Each agency receives an independent request, deadline, conversation, disposition, and appeal record.": "Magpadala ng isang FOI submission sa isa o higit pang ahensiya ng pamahalaan. Bawat ahensiya ay may hiwalay na kahilingan, takdang petsa, usapan, desisyon, at tala ng apela.",
    "Start an FOI request": "Magsimula ng FOI request",
    "How it works": "Paano ito gumagana",
    "Describe the records you need.": "Ilarawan ang mga rekord na kailangan mo.",
    "Search and select up to 10 agencies.": "Maghanap at pumili ng hanggang 10 ahensiya.",
    "Review every field and submit.": "Suriin ang bawat field bago isumite.",
    "Keep each reference number and track the official response.": "Itago ang bawat reference number at subaybayan ang opisyal na tugon.",
    "Designed for gradual migration": "Dinisenyo para sa unti-unting paglipat",
    "The existing government workflow can continue while selected agencies move to two-way case management, deterministic triage, a Kanban board, audit history, and optional isolated AI assistance.": "Maaaring magpatuloy ang kasalukuyang proseso ng pamahalaan habang lumilipat ang mga piling ahensiya sa two-way case management, deterministic triage, Kanban board, audit history, at opsiyonal na hiwalay na AI assistance.",
    "Low operating cost": "Mababang gastos sa pagpapatakbo",
    "The public frontend consists of static files on GitHub Pages. A compact Rust service and SQLite database run the application backend on a small VPS. This keeps the MVP simple to operate while retaining backups, audit events, health checks, and a path to scale.": "Mga static file sa GitHub Pages ang pampublikong frontend. Ang backend ay isang maliit na Rust service at SQLite database sa maliit na VPS. Simple itong patakbuhin habang may backups, audit events, health checks, at daan para lumaki ang serbisyo.",
    "About iTanongMo": "Tungkol sa iTanongMo",
    "iTanongMo is an MVP for gradually replacing one-way email forwarding with traceable two-way FOI case management.": "Ang iTanongMo ay MVP para unti-unting palitan ang one-way email forwarding ng nasusubaybayang two-way FOI case management.",
    "For citizens": "Para sa mamamayan",
    "Create a request, choose several agencies when needed, receive separate references, and follow public status, messages, and decisions.": "Gumawa ng kahilingan, pumili ng ilang ahensiya kung kailangan, tumanggap ng magkakahiwalay na reference, at subaybayan ang pampublikong status, mensahe, at desisyon.",
    "For government offices": "Para sa mga tanggapan ng pamahalaan",
    "Receive requests in a Kanban workflow, keep an audit history, use recall-oriented deterministic triage, and optionally ask an isolated AI model for draft assistance. Officials remain accountable for every sent response and disposition.": "Tumanggap ng mga kahilingan sa Kanban workflow, magpanatili ng audit history, gumamit ng deterministic triage na nakatuon sa recall, at opsiyonal na kumuha ng draft mula sa hiwalay na AI model. Pananagutan pa rin ng opisyal ang bawat tugon at desisyon.",
    "Canary rollout": "Limitadong paglulunsad",
    "Agencies and users can move in controlled groups. The legacy Rust-rendered path remains available while the static GitHub Pages frontend and versioned Rust API are validated.": "Maaaring lumipat ang mga ahensiya at user sa kontroladong grupo. Magagamit pa rin ang lumang Rust-rendered path habang sinusuri ang static GitHub Pages frontend at versioned Rust API.",
    "Maintenance proposal": "Panukala sa pagpapanatili",
    "Static hosting removes a frontend server and build runtime. One Rust binary, SQLite in WAL mode, scheduled encrypted off-site-ready backups, health checks, and bounded logs fit a small VPS. Growth can later move files or the database without changing the public API contract.": "Inaalis ng static hosting ang frontend server at build runtime. Kasya sa maliit na VPS ang isang Rust binary, SQLite sa WAL mode, nakaiskedyul na encrypted backups, health checks, at limitadong logs. Maaari pang ilipat ang files o database habang lumalaki ang serbisyo nang hindi binabago ang public API contract.",
    "Make an FOI request": "Maghain ng FOI request",
    "Use AI to prepare a draft": "Gumamit ng AI para maghanda ng draft",
    "Ask AI": "Magtanong sa AI",
    "What does the AI do?": "Ano ang ginagawa ng AI?",
    "Your contact details": "Iyong contact details",
    "Name": "Pangalan",
    "Email": "Email",
    "Government agencies": "Mga ahensiya ng pamahalaan",
    "Search departments": "Maghanap ng departamento",
    "Department": "Departamento",
    "Add department": "Idagdag ang departamento",
    "Selected agencies": "Mga napiling ahensiya",
    "0 of 10 agencies selected.": "0 sa 10 ahensiya ang napili.",
    "Records requested": "Mga hinihiling na rekord",
    "Title": "Pamagat",
    "Description": "Paglalarawan",
    "Purpose": "Layunin",
    "Urgency or safety context": "Konteksto ng pagkaapurahan o kaligtasan",
    "I acknowledge the MVP privacy notice and confirm that I reviewed the request.": "Nabasa ko ang abiso sa pagkapribado ng MVP at sinuri ko ang kahilingan.",
    "Submit FOI request": "Isumite ang FOI request",
    "Submission references": "Mga reference ng submission",
    "Requester access token:": "Requester access token:",
    "Keep this requester access token. You need it to reply or file an appeal, and it will not be shown again.": "Itago ang requester access token na ito. Kailangan ito para tumugon o maghain ng apela, at hindi na ito muling ipapakita.",
    "What requester AI does": "Ano ang ginagawa ng requester AI",
    "AI returns one strict JSON draft containing an email body, form values, and suggested departments. Your name and request title remain manual and are never replaced. You must verify every fact and selected department before submission.": "Gumagawa ang AI ng isang mahigpit na JSON draft na may email body, form values, at mga iminungkahing departamento. Mano-mano pa rin ang iyong pangalan at pamagat ng kahilingan. Suriin ang bawat impormasyon at napiling departamento bago isumite.",
    "Close": "Isara",
    "Generate an FOI draft": "Gumawa ng FOI draft",
    "What do you want to ask?": "Ano ang gusto mong itanong?",
    "Describe the records you need, relevant dates, offices, and why you need them.": "Ilarawan ang mga rekord, kaugnay na petsa at tanggapan, at kung bakit kailangan ang mga ito.",
    "Generate JSON draft": "Gumawa ng JSON draft",
    "Email preview": "Preview ng email",
    "Strict JSON response": "Mahigpit na JSON response",
    "Apply values for review": "Ilapat ang values para suriin",
    "Download JSON": "I-download ang JSON",
    "Track an FOI request": "Subaybayan ang FOI request",
    "Request reference": "Reference ng kahilingan",
    "Requester access token": "Requester access token",
    "You can view public status without a token. You need the token to reply or file an appeal.": "Makikita ang pampublikong status kahit walang token. Kailangan ang token para tumugon o maghain ng apela.",
    "View status": "Tingnan ang status",
    "Requester actions": "Mga aksiyon ng requester",
    "Public reply": "Pampublikong tugon",
    "Send reply": "Ipadala ang tugon",
    "Appeal reason": "Dahilan ng apela",
    "File appeal": "Maghain ng apela",
    "One public information service": "Isang serbisyo para sa pampublikong impormasyon",
    "Access the right side of every request": "Pumasok sa tamang portal para sa bawat kahilingan",
    "Citizens can submit and track requests. Government teams can triage cases, coordinate action, and send clear responses.": "Maaaring maghain at sumubaybay ng kahilingan ang mamamayan. Maaaring mag-triage ng kaso, mag-ugnay ng aksiyon, at magpadala ng malinaw na tugon ang mga kawani ng pamahalaan.",
    "Request information and follow every update.": "Humiling ng impormasyon at subaybayan ang bawat update.",
    "Review, triage, and respond from one workspace.": "Suriin, i-triage, at sagutin ang mga kahilingan sa iisang workspace.",
    "Secure access": "Ligtas na pagpasok",
    "Sign in to iTanongMo": "Mag-sign in sa iTanongMo",
    "Email address": "Email address",
    "Password": "Password",
    "Enter your password": "Ilagay ang iyong password",
    "Continue to portal": "Magpatuloy sa portal",
    "Demo accounts": "Mga demo account",
    "Citizen:": "Mamamayan:",
    "Government:": "Pamahalaan:",
    "Good day,": "Magandang araw,",
    "Citizen": "Mamamayan",
    "Request public records and keep every government response in one clear timeline.": "Humiling ng mga pampublikong rekord at tingnan ang bawat tugon ng pamahalaan sa iisang malinaw na timeline.",
    "Sign out": "Mag-sign out",
    "Request summary": "Buod ng mga kahilingan",
    "Total requests": "Kabuuang kahilingan",
    "Active reviews": "Kasalukuyang sinusuri",
    "Responses received": "Mga natanggap na tugon",
    "New request": "Bagong kahilingan",
    "Ask for public information": "Humiling ng pampublikong impormasyon",
    "Government agency": "Ahensiya ng pamahalaan",
    "Contact email": "Contact email",
    "Request title": "Pamagat ng kahilingan",
    "What records do you need?": "Anong mga rekord ang kailangan mo?",
    "Request details": "Mga detalye ng kahilingan",
    "Include dates, offices, or specific records.": "Isama ang mga petsa, tanggapan, o partikular na rekord.",
    "Urgency or safety context (optional)": "Konteksto ng pagkaapurahan o kaligtasan (opsiyonal)",
    "I acknowledge the demo privacy notice.": "Nabasa ko ang abiso sa pagkapribado para sa demo.",
    "Submit request": "Isumite ang kahilingan",
    "Tracking": "Pagsubaybay",
    "Your requests": "Iyong mga kahilingan",
    "Refresh": "I-refresh",
    "Information request workspace": "Workspace ng mga kahilingan sa impormasyon",
    "Review incoming cases, apply consistent triage, and keep citizens informed.": "Suriin ang mga bagong kaso, gumamit ng pare-parehong triage, at panatilihing may alam ang mamamayan.",
    "Case summary": "Buod ng mga kaso",
    "New cases": "Mga bagong kaso",
    "Under review": "Sinusuri",
    "For response": "Para sa tugon",
    "Case management": "Pamamahala ng kaso",
    "Request board": "Board ng mga kahilingan",
    "Selected request": "Napiling kahilingan",
    "Choose a request": "Pumili ng kahilingan",
    "Triage case": "I-triage ang kaso",
    "Category": "Kategorya",
    "Infrastructure and procurement": "Imprastraktura at procurement",
    "Agriculture and food security": "Agrikultura at seguridad sa pagkain",
    "Public health": "Pampublikong kalusugan",
    "Education": "Edukasyon",
    "General public records": "Pangkalahatang pampublikong rekord",
    "Assign to unit": "I-assign sa unit",
    "Apply triage": "Ilapat ang triage",
    "Route to unit": "I-route sa unit",
    "Reply to citizen": "Tumugon sa mamamayan",
    "Public response": "Pampublikong tugon",
    "Write a clear status update or response.": "Sumulat ng malinaw na status update o tugon.",
    "Send response": "Ipadala ang tugon",
    "Government workspace": "Workspace ng pamahalaan",
    "Demo persona": "Demo persona",
    "These are public demonstration identities, not verified government accounts.": "Mga pampublikong demo identity lamang ito, hindi verified government accounts.",
    "Persona": "Persona",
    "Open workspace": "Buksan ang workspace",
    "Workspace": "Workspace",
    "Refresh notifications": "I-refresh ang notifications",
    "Download CSV report": "I-download ang CSV report",
    "Notifications": "Mga notification",
    "Request": "Kahilingan",
    "Workflow action": "Aksiyon sa workflow",
    "Action": "Aksiyon",
    "Destination unit": "Tatanggap na unit",
    "Public reason or note": "Pampublikong dahilan o tala",
    "Apply action": "Ilapat ang aksiyon",
    "Conversation or internal note": "Usapan o internal na tala",
    "Visibility": "Visibility",
    "Public": "Pampubliko",
    "Internal": "Internal",
    "Message": "Mensahe",
    "Save message": "I-save ang mensahe",
    "Human triage": "Triage ng tao",
    "Local classifier pilot: model scores are advisory and uncalibrated for Filipino/Taglish FOI. Nothing is saved until a reviewer selects Save triage.": "Pilot lamang ang local classifier: gabay ang model scores at hindi pa calibrated para sa Filipino/Taglish FOI. Walang ise-save hangga't hindi pinipili ng reviewer ang Save triage.",
    "Run local classifier": "Patakbuhin ang local classifier",
    "Copy suggestions to review form": "Kopyahin ang mga mungkahi sa review form",
    "Categories, comma separated": "Mga kategorya, paghiwalayin ng kuwit",
    "Save triage": "I-save ang triage",
    "Government AI draft": "AI draft ng pamahalaan",
    "Verified facts and record references": "Mga napatunayang impormasyon at reference ng rekord",
    "Queue draft": "I-queue ang draft",
    "Completed draft": "Natapos na draft",
    "Select a completed draft": "Pumili ng natapos na draft",
    "Reviewed response": "Nasuring tugon",
    "I verified every factual claim and record reference.": "Napatunayan ko ang bawat factual claim at reference ng rekord.",
    "Publish reviewed response": "I-publish ang nasuring tugon",
    "Release document": "Maglabas ng dokumento",
    "File": "File",
    "Case data": "Data ng kaso",
    "Privacy notice for the MVP": "Abiso sa pagkapribado para sa MVP",
    "Requester AI": "Requester AI",
    "Government AI": "AI ng pamahalaan",
    "Government-side AI uses an isolated configured model and creates reviewable drafts. A government official must verify and explicitly publish a response.": "Gumagamit ang government-side AI ng hiwalay na naka-configure na model at gumagawa ng draft na masusuri. Kailangang patunayan at tahasang i-publish ng opisyal ng pamahalaan ang tugon.",
    "MVP limitation": "Limitasyon ng MVP",
    "Identity verification and eGov integration are post-MVP controls. This deployment must remain a canary until those controls and a production privacy assessment are complete.": "Pagkatapos ng MVP pa ang identity verification at eGov integration. Dapat manatiling limitadong pilot ang deployment na ito hanggang makumpleto ang mga kontrol na iyon at production privacy assessment.",
    "The MVP uses semantic headings, labels, fieldsets, status regions, keyboard-operable controls, and text that remains usable without visual styling.": "Gumagamit ang MVP ng semantic headings, labels, fieldsets, status regions, controls na magagamit sa keyboard, at text na nababasa kahit walang visual styling.",
    "If a form, status update, document, or government response is inaccessible, contact the responsible FOI office and include the page or request reference plus the format you need.": "Kung hindi ma-access ang form, status update, dokumento, o tugon ng pamahalaan, makipag-ugnayan sa kinauukulang FOI office at ibigay ang pahina o request reference at ang format na kailangan mo.",
    "Current work": "Kasalukuyang gawain",
    "Formal WCAG testing, assistive-technology testing, Filipino-language review, and accessible document remediation remain release-gate work before broad production rollout.": "Kailangan pa ang pormal na WCAG testing, assistive-technology testing, pagsusuri ng wikang Filipino, at pag-ayos ng accessibility ng mga dokumento bago ang malawakang production rollout.",
    "The API is not configured. Set ITANONGMO_API_BASE_URL for this deployment.": "Hindi pa naka-configure ang API. I-set ang ITANONGMO_API_BASE_URL para sa deployment na ito.",
    "The API is not configured for this deployment.": "Hindi pa naka-configure ang API para sa deployment na ito.",
    "Checking your account…": "Sinusuri ang iyong account…",
    "Submit a request to begin tracking.": "Maghain ng kahilingan para masimulang subaybayan ito.",
    "Submitting your request…": "Isinusumite ang iyong kahilingan…",
    "The API returned no request reference.": "Walang ibinalik na request reference ang API.",
    "Unassigned": "Hindi pa naka-assign",
    "No cases in this stage.": "Walang kaso sa yugtong ito.",
    "New": "Bago",
    "No cases assigned to your agency": "Walang kasong nakatalaga sa iyong ahensiya",
    "Saving triage…": "Sine-save ang triage…",
    "Triage saved for human review.": "Na-save ang triage para suriin ng tao.",
    "Routing case…": "Niruruta ang kaso…",
    "Case routed to the selected unit.": "Na-route ang kaso sa napiling unit.",
    "Sending response…": "Ipinapadala ang tugon…",
    "Public response sent.": "Naipadala ang pampublikong tugon.",
    "Could not reach the API. Check its URL, TLS, and cookie settings.": "Hindi maabot ang API. Suriin ang URL, TLS, at cookie settings nito.",
    "Authentication is required.": "Kailangang mag-sign in.",
    "The session has expired.": "Nag-expire na ang session.",
    "Invalid email or password.": "Mali ang email o password.",
    "Invalid CSRF token.": "Mali ang CSRF token.",
    "Email and password are required.": "Kailangan ang email at password.",
    "AI service is temporarily unavailable": "Pansamantalang hindi magamit ang AI service",
    "Database operation failed": "Nabigo ang database operation",
    "Internal operation failed": "Nabigo ang internal operation",
    "Remove": "Alisin",
    "No matching department": "Walang katugmang departamento",
    "Maximum of 10 departments per submission.": "Hanggang 10 departamento lamang sa bawat submission.",
    "Clipboard paste is unavailable. Paste the key manually into the password field.": "Hindi magamit ang clipboard paste. Manu-manong i-paste ang key sa password field.",
    "Generating one strict JSON draft…": "Gumagawa ng isang mahigpit na JSON draft…",
    "Review the JSON, departments, and email preview before applying.": "Suriin ang JSON, mga departamento, at email preview bago ilapat.",
    "Applied for review. Name and title were preserved; verify every field and department.": "Nailapat para suriin. Hindi binago ang pangalan at pamagat; suriin ang bawat field at departamento.",
    "Select at least one agency.": "Pumili ng kahit isang ahensiya.",
    "Submitting…": "Isinusumite…",
    "Saving…": "Sine-save…",
    "Saved.": "Na-save.",
    "Classifier output is advisory. Review before saving triage.": "Gabay lamang ang output ng classifier. Suriin bago i-save ang triage.",
    "Loading local model and classifying. The first download is about 107 MB.": "Nilo-load ang local model at nagki-classify. Humigit-kumulang 107 MB ang unang download.",
    "Local classifier timed out.": "Nag-timeout ang local classifier.",
    "Local classifier worker failed.": "Nag-fail ang local classifier worker.",
    "Suggestions copied to the form. Review and select Save triage to record them.": "Nakopya ang mga mungkahi sa form. Suriin at piliin ang Save triage para maitala ang mga ito.",
    "Describe what you want to ask in 20–8,000 characters.": "Ilarawan ang itatanong mo sa 20–8,000 character.",
    "submitted": "naisumite",
    "acknowledged": "natanggap",
    "clarification needed": "kailangan ng paglilinaw",
    "routed": "na-route",
    "in review": "sinusuri",
    "extended": "pinalawig",
    "closed": "isinara",
    "granted": "pinagbigyan",
    "partially granted": "bahagyang pinagbigyan",
    "denied": "tinanggihan",
  });

  const existingFilipino = Object.freeze({
    "Hindi pa configured ang HTTPS Rust API.": "The HTTPS Rust API is not configured yet.",
    "Hindi ma-load ang agency directory.": "Could not load the agency directory.",
    "Invalid ang agency directory.": "The agency directory is invalid.",
    "Kailangan ang request reference at requester access token.": "The request reference and requester access token are required.",
    "Kailangan ang public HTTPS Rust API URL para buksan ang government workspace.": "A public HTTPS Rust API URL is required to open the government workspace.",
    "Pumili muna ng request.": "Choose a request first.",
    "Pumili ng document.": "Choose a document.",
    "Invalid ang stored AI draft JSON; huwag itong i-publish.": "The stored AI draft JSON is invalid; do not publish it.",
    "Pumili ng completed AI draft.": "Choose a completed AI draft.",
    "Kumpirmahin muna ang factual review.": "Confirm the factual review first.",
    "I-set ang HTTPS Rust API URL sa GitHub repository variable ITANONGMO_API_BASE_URL.": "Set the HTTPS Rust API URL in the GitHub repository variable ITANONGMO_API_BASE_URL.",
    "Hindi nakita ang hinahanap na record": "The requested record was not found",
    "Wala kang pahintulot para sa action na ito": "You are not permitted to perform this action",
    "Masyadong maraming request; subukan muli mamaya": "Too many requests; try again later",
  });

  const dynamicFilipino = Object.freeze([
    [/^(\d+) of 10 agencies selected\.$/, (_, count) => `${count} sa 10 ahensiya ang napili.`],
    [/^(\d+) matching departments\.$/, (_, count) => `${count} katugmang departamento.`],
    [/^Submitted batch (.+)\.$/, (_, reference) => `Naisumite ang batch ${reference}.`],
    [/^Submitted (.+)\. Your batch reference is (.+)\.$/, (_, reference, batch) =>
      `Naisumite ang ${reference}. Ang batch reference mo ay ${batch}.`],
    [/^Due (.+)$/, (_, date) => `Takdang petsa: ${date}`],
    [/^(.+) · Due (.+)$/, (_, agency, date) => `${agency} · Takdang petsa: ${date}`],
    [/^API request failed \((\d+)\)\.$/, (_, status) => `Nabigo ang API request (${status}).`],
    [/^Request failed \((\d+)\)\.$/, (_, status) => `Nabigo ang request (${status}).`],
  ]);

  let language = readPreference();
  const originals = new WeakMap();

  // Mental model: keep the English source for each UI node, then render its selected translation.
  function translate(root = document.documentElement) {
    document.documentElement.lang = language === "fil" ? "fil" : "en";
    translateElement(root);
    for (const element of root.querySelectorAll("*")) translateElement(element);
  }

  function translateElement(element) {
    if (element.matches("script, style, pre, code, textarea") ||
        element.closest?.("[data-language-ignore]")) return;
    for (const node of element.childNodes) {
      if (node.nodeType === Node.TEXT_NODE) translateText(node);
    }
    for (const name of ["placeholder", "title", "aria-label"]) {
      if (element.hasAttribute(name)) translateAttribute(element, name);
    }
  }

  function translateText(node) {
    const current = node.textContent;
    const old = originals.get(node);
    const source = old && current === old.rendered ? old.source : current;
    const trimmed = source.trim();
    const english = existingFilipino[trimmed] || trimmed;
    const dynamic = dynamicFilipino.find(([pattern]) => pattern.test(english));
    if (!Object.hasOwn(filipino, english) && !Object.hasOwn(existingFilipino, trimmed) && !dynamic) return;
    const selected = language === "fil"
      ? (filipino[english] || (dynamic && english.replace(...dynamic)) || trimmed)
      : english;
    const rendered = source.replace(trimmed, selected);
    if (node.parentElement?.tagName === "OPTION" && !node.parentElement.hasAttribute("value")) {
      node.parentElement.value = trimmed;
    }
    originals.set(node, { source, rendered });
    if (current !== rendered) node.textContent = rendered;
  }

  function translateAttribute(element, name) {
    const current = element.getAttribute(name);
    const key = `${name}:source`;
    const renderedKey = `${name}:rendered`;
    const old = originals.get(element) || {};
    const source = current === old[renderedKey] ? old[key] : current;
    if (!Object.hasOwn(filipino, source)) return;
    const rendered = language === "fil" ? filipino[source] : source;
    originals.set(element, { ...old, [key]: source, [renderedKey]: rendered });
    if (current !== rendered) element.setAttribute(name, rendered);
  }

  function readPreference() {
    try {
      return localStorage.getItem(storageKey) === "fil" ? "fil" : "en";
    } catch {
      return "en";
    }
  }

  function setLanguage(next) {
    if (next !== "en" && next !== "fil") return;
    language = next;
    try { localStorage.setItem(storageKey, next); } catch { /* Browser storage may be disabled. */ }
    translate();
    document.dispatchEvent(new CustomEvent("itanongmo:languagechange", { detail: { language } }));
  }

  window.iTanongMoLanguage = Object.freeze({
    get current() { return language; },
    setLanguage,
    translate,
    text(source) {
      const english = existingFilipino[source] || source;
      return language === "fil" ? (filipino[english] || source) : english;
    },
  });

  function start() {
    translate();
    const observer = new MutationObserver((records) => {
      for (const record of records) {
        if (record.type === "characterData") translateText(record.target);
        else if (record.type === "attributes") translateAttribute(record.target, record.attributeName);
        else for (const node of record.addedNodes) {
          if (node.nodeType === Node.ELEMENT_NODE) translate(node);
          else if (node.nodeType === Node.TEXT_NODE) translateText(node);
        }
      }
    });
    observer.observe(document.body, {
      subtree: true,
      childList: true,
      characterData: true,
      attributes: true,
      attributeFilter: ["placeholder", "title", "aria-label"],
    });
  }

  if (document.body) start();
  else document.addEventListener("DOMContentLoaded", start, { once: true });
})();
