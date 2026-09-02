/**
 * English — the source catalog.
 *
 * This file defines the key set: every other locale is typed against it, so a
 * missing or misspelled key is a compile error rather than a blank label.
 *
 * Only the app's own chrome belongs here. State, event, effect, machine and
 * core names come from the analyzed application through the model and are
 * never translated.
 */

export const en = {
  'app.loading': 'Loading…',

  'toolbar.simulate': 'Simulate',
  'toolbar.stopSimulation': 'Stop simulation',
  'toolbar.relayout': 'Re-layout',
  // Tag *names* are the analyzed app's identifiers and stay untranslated;
  // only this chrome around them is localized.
  'toolbar.filterByTag': 'Filter by tag',
  'toolbar.undocumented': 'Undocumented',
  'toolbar.undocumentedHint':
    'Highlight states without documentation — the ones with no /// description in the analyzed source',

  // Two complete sentences rather than one template: the interpolated word
  // would need adjective agreement in some locales.
  'themeToggle.switchToLight': 'Switch to light mode',
  'themeToggle.switchToDark': 'Switch to dark mode',
  // `language` is a locale's endonym ("Português (Brasil)") — a proper name,
  // so it is substituted untranslated.
  'localeToggle.switchTo': 'Switch to {language}',

  'sidebar.cores': 'Cores',
  'sidebar.expand': 'Expand',
  'sidebar.collapse': 'Collapse',
  // `name` is a machine, family or state name read out of the analyzed
  // application — a proper name, substituted untranslated.
  'sidebar.hideFromCanvas': 'Hide {name} from the canvas',
  'sidebar.showOnCanvas': 'Show {name} on the canvas',
  // What a row's name does, as opposed to its checkbox: everything else the
  // core declares leaves the canvas.
  'sidebar.showOnly': 'Show only {name} on the canvas',
  'sidebar.showAll': 'Show all states',
  // "GitHub" is a proper name and stays as it is; only the description of
  // where the link goes is translated.
  'sidebar.sourceCode': 'Source code on GitHub',

  'inspector.title': 'Inspector',
  'inspector.empty': 'Select a state or a transition.',
  'inspector.incoming': 'Incoming',
  'inspector.outgoing': 'Outgoing',
  'inspector.effects': 'Effects',
  // A union over the incoming transitions: "some of these", never "all".
  'inspector.entryEffects': 'Effects on entry',
  // The return leg of the loop: which events the shell can send back.
  'inspector.answersWith': 'answers with',
  // The request sits on a branch the transition does not imply.
  'inspector.conditional': 'may',
  'inspector.none': '—',
  'inspector.tags': 'Tags',
  'inspector.aboutMachine': 'About this machine',

  'badge.initial': 'initial',
  'badge.failure': 'failure',
  'badge.deprecated': 'deprecated',
  'badge.final': 'final',

  // Prose standing in for the `"*"` wildcard sentinel carried by the model.
  'state.anyState': 'any state',
  'state.anyStateRuntime': 'any state (runtime)',

  'simulation.title': 'Simulation',
  'simulation.unknownState': '?',
  'simulation.sendEvent': 'Send event',
  'simulation.noEvents': 'No events can fire from here.',
  'simulation.runtimeTargetNote':
    'These land on a state decided at runtime, so the replay cannot follow them.',
  'simulation.inFlight': 'Waiting for the shell',
  'simulation.fromShell': 'from the shell',
  'simulation.inertAnswerNote':
    'The shell can send these back, and no transition here handles them: real behavior that changes no state.',
  'simulation.trail': 'Trail',
  // Per step: what firing it asked the shell to do.
  'simulation.requested': 'requested',
  // A request on a branch the replay does not evaluate: it cannot claim it ran.
  'simulation.mayHave': 'may have',
  'simulation.aheadNote':
    'Steps the replay was rewound past. They are kept, not taken: firing the same event again walks into them, and a different move replaces them.',
  'simulation.nothingFired': 'Nothing fired yet.',
  'simulation.restart': 'Restart',

  // React Flow renders its own controls; these override its English defaults.
  'graph.a11y.controls': 'Control Panel',
  'graph.a11y.zoomIn': 'Zoom in',
  'graph.a11y.zoomOut': 'Zoom out',
  'graph.a11y.fitView': 'Fit view',

  // Proposal Mode keys
  'proposal.badge': 'Proposal',
  'proposal.proposeChanges': 'Propose changes',
  'proposal.proposeChangesHint': 'Draft modifications to state machines and generate implementation briefing',
  'proposal.review': 'Review',
  'proposal.disabledInSimulation': 'Proposal mode cannot be activated during simulation',
  'proposal.disabledInProposal': 'Simulation mode cannot be activated while proposing changes',
  'proposal.stale': 'Stale',
  'proposal.staleHint': 'The base source code changed since this proposal was drafted',
  'proposal.addEffect': 'Add effect',
  'proposal.removeTransition': 'Remove transition',
  'proposal.addTransition': 'Add transition',
  'proposal.effectName': 'Effect name',
  'proposal.capability': 'Capability (optional)',
  'proposal.conditionalLabel': 'Conditional effect ("may request")',
  'proposal.answers': 'Shell callback events (comma separated)',
  'proposal.save': 'Save',
  'proposal.cancel': 'Cancel',
  'proposal.eventName': 'Event name',
  'proposal.targetState': 'Target state',
  'proposal.createTransition': 'Create transition',
  'proposal.editDoc': 'Edit documentation',
  'proposal.noDoc': 'No documentation authored on this state.',
  'proposal.docPlaceholder': 'Authored state documentation...',
  'proposal.wildcardReadOnly': 'Wildcard transitions are read-only in v1 proposal mode.',
  'proposal.reviewTitle': 'Review Proposed Changes',
  'proposal.tabBriefing': 'Markdown Briefing',
  'proposal.tabScaffolding': 'Rust Scaffolding',
  'proposal.tabJson': 'JSON ChangeSet',
  'proposal.authorNoteLabel': 'Author Note / Context (included in Briefing):',
  'proposal.notePlaceholder': 'Add any notes for devs/LLMs implementing these changes...',
  'proposal.copyBriefing': 'Copy Briefing Markdown',
  'proposal.copyScaffolding': 'Copy Rust Scaffolding',
  'proposal.copyJson': 'Copy JSON',
  'proposal.copied': 'Copied to clipboard!',
  'proposal.close': 'Close',
  'proposal.confirmExit': 'You have proposed changes. Are you sure you want to exit proposal mode?',
  'proposal.discard': 'Reset',
  'proposal.discardTitle': 'Discard all proposed changes',
  'proposal.confirmDiscard': 'Are you sure you want to discard all proposed changes? This action cannot be undone.',
  'toolbar.showEffects': 'Effects',
  'toolbar.showEffectsHint': 'Show effects on edges',
};

/** Every key the UI may ask for. */
export type MessageKey = keyof typeof en;

/** The shape every locale must fill completely. */
export type Catalog = Record<MessageKey, string>;
