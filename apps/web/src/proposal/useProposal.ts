import { useCallback, useEffect, useMemo, useState } from 'react';
import type { DomainCore } from '../domain/types';
import { applyProposal } from './apply';
import { computeChangeSet } from './diff';
import { computeCoreHash, discardProposal, loadProposal, saveProposal } from './storage';
import type { ChangeSet, Proposal, ProposalOp } from './types';

const MAX_UNDO_DEPTH = 50;

export interface UseProposalReturn {
  isProposing: boolean;
  proposal: Proposal | null;
  projectedCore: DomainCore | null;
  changeSet: ChangeSet | null;
  isDirty: boolean;
  isStale: boolean;
  canUndo: boolean;
  canRedo: boolean;
  toggleProposalMode: () => void;
  addOp: (op: ProposalOp) => void;
  undo: () => void;
  redo: () => void;
  setNote: (note: string) => void;
  discard: () => void;
  rebase: () => void;
}

export function useProposal(baseCore: DomainCore | null): UseProposalReturn {
  const [isProposing, setIsProposing] = useState<boolean>(false);
  const [proposal, setProposal] = useState<Proposal | null>(null);

  const currentBaseHash = useMemo(() => {
    return baseCore ? computeCoreHash(baseCore) : '';
  }, [baseCore]);

  // Load existing proposal on core change
  useEffect(() => {
    if (!baseCore) {
      setProposal(null);
      return;
    }
    const saved = loadProposal(baseCore.id);
    if (saved) {
      setProposal(saved);
    }
  }, [baseCore]);

  // Auto-save proposal when modified
  useEffect(() => {
    if (proposal && proposal.ops.length > 0) {
      saveProposal(proposal);
    }
  }, [proposal]);

  // Staleness check: does proposal baseHash match current baseHash?
  const isStale = useMemo(() => {
    if (!proposal || !currentBaseHash) return false;
    return proposal.baseHash !== currentBaseHash;
  }, [proposal, currentBaseHash]);

  const toggleProposalMode = useCallback(() => {
    setIsProposing((prev) => {
      const next = !prev;
      if (next && !proposal && baseCore) {
        // Initialize new proposal
        setProposal({
          coreId: baseCore.id,
          ops: [],
          undoCursor: 0,
          baseHash: currentBaseHash,
          note: '',
        });
      }
      return next;
    });
  }, [proposal, baseCore, currentBaseHash]);

  const addOp = useCallback(
    (op: ProposalOp) => {
      if (!baseCore || isStale) return;

      setProposal((prev) => {
        const currentOps = prev ? prev.ops.slice(0, prev.undoCursor) : [];
        const newOps = [...currentOps, op];
        if (newOps.length > MAX_UNDO_DEPTH) {
          newOps.shift();
        }
        return {
          coreId: baseCore.id,
          ops: newOps,
          undoCursor: newOps.length,
          baseHash: currentBaseHash,
          note: prev?.note || '',
        };
      });
    },
    [baseCore, isStale, currentBaseHash]
  );

  const undo = useCallback(() => {
    setProposal((prev) => {
      if (!prev || prev.undoCursor <= 0) return prev;
      return {
        ...prev,
        undoCursor: prev.undoCursor - 1,
      };
    });
  }, []);

  const redo = useCallback(() => {
    setProposal((prev) => {
      if (!prev || prev.undoCursor >= prev.ops.length) return prev;
      return {
        ...prev,
        undoCursor: prev.undoCursor + 1,
      };
    });
  }, []);

  const setNote = useCallback((note: string) => {
    setProposal((prev) => (prev ? { ...prev, note } : prev));
  }, []);

  const discard = useCallback(() => {
    if (baseCore) {
      discardProposal(baseCore.id);
    }
    setProposal(null);
    setIsProposing(false);
  }, [baseCore]);

  const rebase = useCallback(() => {
    if (!proposal || !baseCore) return;
    // Rebase: update baseHash to currentBaseHash
    setProposal({
      ...proposal,
      baseHash: currentBaseHash,
    });
  }, [proposal, baseCore, currentBaseHash]);

  // Derive projected DomainCore
  const projectedCore = useMemo(() => {
    if (!baseCore || !isProposing || !proposal) return baseCore;
    return applyProposal(baseCore, proposal);
  }, [baseCore, isProposing, proposal]);

  // Derive ChangeSet
  const changeSet = useMemo(() => {
    if (!baseCore || !projectedCore || !isProposing) return null;
    return computeChangeSet(baseCore, projectedCore);
  }, [baseCore, projectedCore, isProposing]);

  const isDirty = useMemo(() => {
    return changeSet ? changeSet.totalChanges > 0 : false;
  }, [changeSet]);

  const canUndo = proposal ? proposal.undoCursor > 0 : false;
  const canRedo = proposal ? proposal.undoCursor < proposal.ops.length : false;

  // Keyboard shortcut listener for Ctrl+Z / Ctrl+Y / Cmd+Shift+Z
  useEffect(() => {
    if (!isProposing) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        (e.target as HTMLElement).tagName === 'INPUT' ||
        (e.target as HTMLElement).tagName === 'TEXTAREA'
      ) {
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
        if (e.shiftKey) {
          e.preventDefault();
          redo();
        } else {
          e.preventDefault();
          undo();
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
        e.preventDefault();
        redo();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isProposing, undo, redo]);

  return {
    isProposing,
    proposal,
    projectedCore,
    changeSet,
    isDirty,
    isStale,
    canUndo,
    canRedo,
    toggleProposalMode,
    addOp,
    undo,
    redo,
    setNote,
    discard,
    rebase,
  };
}
