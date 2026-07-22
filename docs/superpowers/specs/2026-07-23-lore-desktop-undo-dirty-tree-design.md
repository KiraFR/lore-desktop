# Lore Desktop — Undo commit avec arbre sale

Validé par Jimmy le 2026-07-23 : « on pourrait undo quand même et si on undo un fichier et que le fichier est dans l'arbre des changes, on l'écrase avec modal ».

## Comportement actuel

« Undo commit » (History, dernier commit local non poussé) est masqué dès que l'arbre de travail a des changements en attente (`canUndo` exige un arbre propre, History.svelte ~l.100) — la garde servait à ce que l'undo recapture exactement le commit.

## Nouveau comportement

1. **La garde arbre-propre saute** : `canUndo` = dernier commit local non poussé, point (les autres conditions existantes — pas de merge en cours, etc. — restent telles quelles si présentes).
2. **Sans recouvrement** (aucun fichier du commit n'a de modification en attente) : confirmation actuelle inchangée ; les changements en attente sur d'AUTRES fichiers survivent tels quels.
3. **Avec recouvrement** (fichier(s) du commit ayant AUSSI une modification en attente) : modale d'avertissement AVANT toute action — `confirmAction` avec un message explicite : « Undoing this commit will overwrite your pending changes to N file(s): a, b, c… » (lister jusqu'à 5 chemins, puis « and N more ») + le message d'undo habituel. Sur confirmation : la modification en attente de chaque fichier recouvrant est ÉCRASÉE par la version du commit annulé, puis l'undo standard s'exécute. Sur annulation : rien ne bouge.

## Mécanique (à valider contre le code actuel par l'implémenteur)

- Les fichiers du commit sélectionné s'obtiennent via les données déjà chargées du commit (files de l'entrée history) ; le recouvrement = ∩ avec `repo.status.files` (chemins).
- **Écrasement = reset des fichiers recouvrants AVANT l'undo** : tant que le tip est encore le commit à annuler, `discardFile`/`lore reset <chemin absolu>` ramène le fichier à la version committée (= la version du commit annulé) — exactement l'« écrasement » voulu, sans besoin de contenu à une révision. Puis l'undo existant (`undoCommit(parents[0])`) s'exécute ; les fichiers du commit redeviennent des changements en attente.
- Vérifier si le chemin Rust de l'undo impose lui aussi un arbre propre ; si oui, adapter (la nouvelle sémantique est celle décrite ici).
- Si l'undo échoue APRÈS les resets, message d'erreur honnête (les resets ne sont pas annulables) — toast existant.
- Logique de recouvrement/troncature du message en module TS pur testable (ex. `undoOverlap.ts`) ; parité mock (le mock doit exercer le scénario recouvrement).

## Vérification

vitest (module pur + adaptations), cargo si le Rust bouge, preview mock : commit → modifier un des fichiers committés + un fichier hors commit → Undo : modale liste le recouvrant, confirme → le recouvrant revient à la version committée dans Changes, le hors-commit garde sa modification. ⚠️ preview : la modale native s'affiche sur l'écran de Jimmy — l'ANNULER seulement en préview auto ; le flux confirmé se vérifie via les tests + le mock DOM sans cliquer le bouton destructif, ou se fait valider par Jimmy en réel.
