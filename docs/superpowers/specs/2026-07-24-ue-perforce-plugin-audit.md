# Audit du plugin Perforce d'Unreal Engine 5.7 — préparation du plugin Lore

**Date** : 2026-07-24
**Objectif** : inventorier exhaustivement ce que fait l'intégration Perforce/Revision Control d'UE 5.7.2, et mapper chaque capacité sur le CLI `lore` actuel, pour définir le périmètre d'un plugin « Lore Source Control » à parité artiste avec Perforce.

**Sources auditées (lecture seule)** :
- `C:\Program Files\Epic Games\UE_5.7\Engine\Plugins\Developer\PerforceSourceControl\` (provider Perforce)
- `C:\Program Files\Epic Games\UE_5.7\Engine\Source\Developer\SourceControl\` (framework `ISourceControlProvider`)
- `C:\Program Files\Epic Games\UE_5.7\Engine\Source\Developer\UncontrolledChangelists\`
- `C:\Program Files\Epic Games\UE_5.7\Engine\Source\Editor\SourceControlWindows\` (fenêtres Submit/History/Revert/Changelists)
- `C:\Program Files\Epic Games\UE_5.7\Engine\Source\Editor\UnrealEd\` (checkout-on-edit, prompts de save)
- Surface CLI réelle vérifiée via `lore <cmd> --help` (v installée localement)

**Contexte projet** : le projet du studio `Games\FirstPerson` est aujourd'hui configuré sur Perforce (`Saved/Config/WindowsEditor/SourceControlSettings.ini` : `Provider=Perforce`, `Port=ssl:51.68.228.130:1666`, `UserName=super`, `Workspace=Deinos`). Le plugin Lore devra remplacer cette configuration.

Dans ce document, les références de code sont notées `Fichier.cpp:ligne`. Racines abrégées :
- `[P4]` = `Engine\Plugins\Developer\PerforceSourceControl\Source\PerforceSourceControl\Private`
- `[SC]` = `Engine\Source\Developer\SourceControl`
- `[SCW]` = `Engine\Source\Editor\SourceControlWindows`
- `[UED]` = `Engine\Source\Editor\UnrealEd`

---

## 1. Architecture du framework Revision Control d'UE

Tout provider implémente `ISourceControlProvider` (`[SC]\Public\ISourceControlProvider.h`). Les points d'entrée essentiels :

| API | Réf | Rôle |
|---|---|---|
| `Execute(Operation, Files, Concurrency, CompleteDelegate)` | ISourceControlProvider.h:308-338 | exécute une `ISourceControlOperation` (sync ou async) sur une liste de fichiers et/ou une changelist |
| `GetState(File/Package, EStateCacheUsage)` | ISourceControlProvider.h:263-268 | retourne le `FSourceControlStatePtr` (cache ou forcé) |
| Capacités déclaratives | ISourceControlProvider.h:422-468 | `UsesLocalReadOnlyState()`, `UsesChangelists()`, `UsesUncontrolledChangelists()`, `UsesCheckout()`, `UsesFileRevisions()`, `UsesSnapshots()`, `AllowsDiffAgainstDepot()`, `IsAtLatestRevision()`, `GetNumLocalChanges()` |

C'est via ces booléens que l'éditeur adapte son UI : Perforce répond (PerforceSourceControlProvider.cpp:608-638) `UsesLocalReadOnlyState=true`, `UsesChangelists=true`, `UsesUncontrolledChangelists=true`, `UsesCheckout=true`, `AllowsDiffAgainstDepot=true`. **Un provider Lore choisira ses propres réponses et l'éditeur suivra** (ex. Git répond `UsesCheckout=false` et l'UI masque « Check Out »).

Le provider Perforce mappe chaque nom d'opération sur un worker : `IPerforceSourceControlWorker::RegisterWorkers()` (`[P4]\PerforceSourceControlOperations.cpp:56-88`) — c'est LA liste exhaustive des 29 opérations supportées. Les commandes sont exécutées en file sur un thread (`FPerforceSourceControlCommand`, `[P4]\PerforceSourceControlCommand.cpp`), avec support d'annulation et de reconnexion.

Autres briques du framework à connaître pour le plugin :
- `SourceControlOperationBase.h` : base commune (flags, résultats, logging).
- `SourceControlFileStatusMonitor.h` + `SourceControlAssetDataCache.h` (`[SC]\Public`) : polling périodique en arrière-plan des statuts pour le Content Browser.
- `SSourceControlLogin` (`[SC]\Private\SSourceControlLogin.h:30`) : dialog générique de connexion, qui héberge le widget de settings du provider (pour Perforce : `SPerforceSourceControlSettings.cpp` — port/user/workspace + auto-détection `p4 info`).
- `USourceControlPreferences` (`[SC]\Public\SourceControlPreferences.h`) : options projet (ex. `ShouldDeleteNewFilesOnRevert`, validation de tags dans la description de submit).

---

## 2. Inventaire des opérations (liste EXACTE)

Déclarations des opérations : `[SC]\Public\SourceControlOperations.h`. Workers Perforce : `[P4]\PerforceSourceControlOperations.cpp` (+ `PerforceSourceControlInternalOperations.h` pour les opérations privées). Colonne « Déclencheur » = où l'éditeur la lance au quotidien.

| Opération (FName) | Déclaration | Worker P4 (`Execute`) | Commande p4 | Ce que ça fait / déclencheur éditeur |
|---|---|---|---|---|
| `Connect` | SourceControlOperations.h:28 | PerforceSourceControlOperations.cpp:427 | `p4 client` (validation workspace, :452) | Test/établissement de connexion. Déclenché par le dialog de login (SSourceControlLogin) et au chargement du provider. Porte un mot de passe optionnel. |
| `UpdateStatus` | SourceControlOperations.h:477 | :1907 | `p4 fstat` (:1993-1998), `p4 opened` (:2022), `p4 diff -s` (:2056), `p4 filelog` si historique demandé (:1863) | LE cœur : rafraîchit l'état de chaque fichier (checked out, par qui, à jour ou non, conflits). Options : `bUpdateHistory`, `bGetOpenedOnly`, `bUpdateModifiedState`, `bForceUpdate`. Déclenché partout : ouverture d'asset, tick éditeur après dirty (UnrealEdEngine.cpp:654), Content Browser, fenêtre Submit. |
| `CheckOut` | SourceControlOperations.h:131 | :495 | `p4 edit` (:519) | Ouvre le fichier en édition (le rend inscriptible). Déclencheurs : menu contextuel Content Browser, checkout auto à la modification, prompt de save. |
| `CheckIn` | SourceControlOperations.h:73 | :619 | `p4 reopen` (déplacement dans la CL, :591) puis `p4 submit` (:685) ; `p4 change` pour re-checkout si `bKeepCheckedOut` (:742) | Soumet une changelist avec description. Renvoie un `SuccessMessage` (n° de CL soumise). Déclenché par la fenêtre Submit. |
| `MarkForAdd` | SourceControlOperations.h:239 | :850 | `p4 add` (:901) | Marque des nouveaux fichiers pour ajout. Déclenché à la création/import/duplication d'assets (si option activée) et par la fenêtre Submit pour les fichiers non contrôlés cochés. |
| `Delete` | SourceControlOperations.h:257 | :924 | `p4 delete` (:949) | Marque pour suppression. Déclenché par la suppression d'asset dans l'éditeur (AssetDeleteModel). |
| `Revert` | SourceControlOperations.h:275 | :970 | `p4 revert` (:1011) | Annule les modifications locales. Options : `bIsSoftRevert`, `bIsRevertAll`, `ShouldDeleteNewFilesOnRevert`. Déclenché par le dialog Revert et « Revert all local changes » du status bar. |
| `Sync` | SourceControlOperations.h:401 | :1074 | `p4 sync [rev]` (:1130) | Ramène les fichiers à une révision (head par défaut). Options : révision cible, `bForce`, `bLastSynced`. Déclenché par « Sync Latest » (status bar / menu) et le menu contextuel. |
| `SyncPreview` | SourceControlOperations.h:330 | — (pas de worker P4 enregistré) | — | Prévisualisation d'un sync (fichiers affectés, taille de transfert). Utilisé par d'autres providers (UnrealGameSync-like). |
| `GetFileList` | SourceControlOperations.h:149 | :799 | `p4 files` (:820) | Liste les fichiers du depot sous un chemin (option : inclure les supprimés). |
| `Copy` | SourceControlOperations.h:631 | :2657 | `p4 integrate` + `p4 resolve -ay` (:2681, :2700) ou simple `p4 add` (:2709) selon `ECopyMethod::Branch/Add` (cvar `p4.AlwaysBranchFilesOnCopy` :41-47) | Copie/branche un fichier avec lien de parenté. Déclenché par duplication/« Save As » d'assets. Depuis UE5.4 le défaut est `Add` (pas de branchement). |
| `Resolve` | SourceControlOperations.h:686 | :2727 | `p4 resolve -ay` (accept yours, :2743) | Marque un conflit comme résolu en gardant la version locale. Déclenché après un sync conflictuel (état `Conflicted`). |
| `ChangeStatus` | Public\PerforceSourceControlChangeStatusOperation.h | :2772 | `p4 diff -s` | Opération spécifique Perforce : liste modifiés/non modifiés d'une CL. |
| `UpdateChangelistsStatus` (`FUpdatePendingChangelistsStatus`) | SourceControlOperations.h:888 | :2375 | `p4 changes -s pending`, `p4 opened` (:2321), `p4 where` (:2362), `p4 describe` pour les shelves (:2494) | Rafraîchit la fenêtre View Changelists (fichiers par CL, shelves). |
| `GetPendingChangelists` | SourceControlOperations.h:704 | (via UpdateChangelistsStatus) | `p4 changes` (:2407) | Liste des CL en attente. |
| `GetSubmittedChangelists` | SourceControlOperations.h:722 | (idem, filtres date/owner/pagination) | `p4 changes -s submitted` | Liste des CL soumises (pagination). |
| `GetChangelistDetails` | SourceControlOperations.h:836 | :3719 | `p4 describe` (:3736) | Détails complets d'une CL (clé/valeur). |
| `NewChangelist` | SourceControlOperations.h:961 | :2829 | `p4 change` (création, :2866) | Créer une CL avec description. Déclenché par la fenêtre Changelists. |
| `DeleteChangelist` | SourceControlOperations.h:1003 | :2919 | `p4 change -d` (:2941) | Supprimer une CL vide. |
| `EditChangelist` | SourceControlOperations.h:1020 | :2975 | `p4 change` (édition description) | Modifier la description d'une CL. |
| `RevertUnchanged` | SourceControlOperations.h:1051 | :3027 | `p4 revert -a` (:3047) | Revert des fichiers ouverts mais non modifiés. |
| `MoveToChangelist` | SourceControlOperations.h:1068 | `FPerforceReopenWorker` :3068 | `p4 reopen -c` | Déplacer des fichiers entre CL (drag & drop dans la fenêtre Changelists). |
| `Shelve` | SourceControlOperations.h:1086 | :3117 | `p4 shelve` (:3175), création de CL au besoin (:3202) | Met de côté les modifications sur le serveur sans submit. |
| `Unshelve` | SourceControlOperations.h:1118 | :3315 | `p4 unshelve` (:3338) | Restaure des fichiers shelvés. |
| `DeleteShelved` | SourceControlOperations.h:1136 | :3251 | `p4 shelve -d` (:3270) | Supprime des fichiers shelvés. |
| `DownloadFile` | SourceControlOperations.h:1155 | :3385 | `p4 print` (:3412, :3456) | Télécharge le contenu d'un fichier du serveur **sans toucher l'état local** (en mémoire ou vers un répertoire cible). Utilisé notamment pour les diffs. |
| `GetFile` | SourceControlOperations.h:1502 | :3772 | `p4 print` | Récupère un fichier à une révision ou depuis un shelve, retourne un nom de fichier temporaire. Utilisé par le diff de changelists/shelves. |
| `GetSourceControlRevisionInfo` | SourceControlOperations.h:1255 | `FPerforceRevisionInfoWorker` :3512 | `p4 changes` (:3541) | CL/date/auteur d'une révision (pour métadonnées, Zen/DDC). |
| `CreateWorkspace` | SourceControlOperations.h:1320 | :3571 | `p4 client` (création de clientspec, stream ou mapping) | Création de workspace (utilisé par les outils d'onboarding type UGS/Horde, pas par l'artiste au quotidien). |
| `DeleteWorkspace` | SourceControlOperations.h:1437 | :3670 | `p4 client -d` (:3692) | Suppression de workspace. |
| `GetWorkspaces` | SourceControlOperations.h:1485 | :2242 | `p4 clients` | Workspaces du host courant (peuplement du dialog de login). |
| `GetProjectWorkspaces` | interne (`PerforceSourceControlInternalOperations.h`) | :2270 | `p4 clients` filtré | Variante filtrée sur le projet. |
| `Where` | SourceControlOperations.h:1553 | :3813 | `p4 where` (:3828) | Mapping chemin local ⇄ chemin depot. |
| `FSave` | SourceControlOperations.h:1583 | — (non implémenté par P4) | — | « Sauver dans le VCS sans check-in » — réservé à d'autres providers. |
| `FSyncPreview`, `FSave` | — | — | — | Déclarés dans le framework mais **sans worker Perforce** : la parité Perforce ne les requiert pas. |

> Note : `FRevert::ShouldDeleteNewFiles()` (SourceControlOperations.h:309) lit `USourceControlPreferences::ShouldDeleteNewFilesOnRevert` — le revert d'un fichier « marked for add » peut supprimer le fichier disque.

---

## 3. États de fichiers & overlays Content Browser

### 3.1 Le modèle d'état

`ISourceControlState` (`[SC]\Public\ISourceControlState.h`) est l'interface interrogée par toute l'UI. Implémentation Perforce : `FPerforceSourceControlState` (`[P4]\PerforceSourceControlState.h`).

États internes Perforce (`EPerforceState`, PerforceSourceControlState.h:13-46) :

| État | Signification |
|---|---|
| `DontCare` | inconnu |
| `CheckedOut` | checked out par moi |
| `ReadOnly` | contrôlé, non checked out (fichier en lecture seule sur disque) |
| `NotInDepot` | nouveau fichier, pas dans le depot |
| `CheckedOutOther` | **checked out par un autre** (noms dans `OtherUserCheckedOut`, :135, multi-utilisateurs séparés par virgules) |
| `Ignore` | ignoré par le SCC |
| `OpenForAdd` | marqué pour ajout |
| `MarkedForDelete` | marqué pour suppression |
| `NotUnderClientRoot` | hors du workspace |
| `Branched` | ouvert pour branch |

S'y ajoutent des axes orthogonaux : `bModifed` (différent du depot), `DepotRevNumber` vs `LocalRevNumber` (→ `IsCurrent()` = « not at head revision » si différents), `PendingResolveInfo` (→ `Conflicted`), `bBinary`, `bExclusiveCheckout` (:159-162), et les infos multi-branches (`HeadBranch`, `CheckedOutBranches`, `OtherUserBranchCheckedOuts`, :168-183).

Les requêtes clés côté UI (PerforceSourceControlState.h:86-107) : `CanCheckIn`, `CanCheckout`, `IsCheckedOut`, `IsCheckedOutOther(FString* Who)`, `IsCheckedOutInOtherBranch`, `IsModifiedInOtherBranch`, `IsCurrent`, `IsSourceControlled`, `IsAdded`, `IsDeleted`, `IsIgnored`, `CanEdit`, `IsModified`, `CanAdd`, `CanDelete`, `CanRevert`.

### 3.2 Parsing p4 → état

`ParseUpdateStatusResults` (`[P4]\PerforceSourceControlOperations.cpp`) lit les records `fstat` : `headRev`/`haveRev` (:1312-1313 → NotAtHead), `otherOpen`/`otherOpen0..N` (:1253, :1291, :1376 → CheckedOutOther + noms), `unresolved` (:1320 → Conflicted), `headType` contenant `+l` (:1512-1514 et :1644 → `bExclusiveCheckout`).

### 3.3 Icônes / overlays

`FPerforceSourceControlState::GetIcon()` (PerforceSourceControlState.cpp:105-148) mappe vers le style set `RevisionControlStyleManager` (`[SC]\Public\RevisionControlStyle\RevisionControlStyle.h`) — ce sont les badges affichés en overlay sur les vignettes du Content Browser et dans le Scene Outliner :

| Icône (`RevisionControl.*`) | Condition |
|---|---|
| `Conflicted` | resolve en attente |
| `NotAtHeadRevision` | version locale en retard sur le depot |
| `CheckedOutByOtherUserOtherBranch` / `ModifiedOtherBranch` | checked out / modifié dans une autre branche (+ badge) |
| `CheckedOut` | checked out par moi (coche rouge) |
| `NotInDepot` | pas encore ajouté (« ? » jaune) |
| `CheckedOutByOtherUser` (+ badge avec nom) | checked out par un autre |
| `OpenForAdd` | marqué pour ajout (« + ») |
| `MarkedForDelete` | marqué pour suppression |
| `Branched` | branché |

`GetDisplayName()`/`GetDisplayTooltip()` fournissent le texte (« Checked out by: X » etc.). Le Content Browser interroge ces états via le cache (`EStateCacheUsage::Use`) alimenté par les `UpdateStatus` en arrière-plan (`SourceControlFileStatusMonitor`).

---

## 4. Points d'intégration éditeur (le quotidien d'un artiste)

### 4.1 Checkout automatique à l'édition

- Hook central : `UUnrealEdEngine::OnPackageDirtyStateUpdated` (`[UED]\Private\UnrealEdEngine.cpp:572`), abonné à `UPackage::PackageDirtyStateChangedEvent` (:119). Chaque package sali est accumulé dans `PackagesDirtiedThisTick` (:596).
- Au tick, `AttemptModifiedPackageNotification` (:614) lance un `FUpdateStatus` **asynchrone** sur ces fichiers (:654), puis `OnSourceControlStateUpdated` (:687) décide : si `CanCheckout()` → checkout auto (si `bAutomaticallyCheckoutOnAssetModification`, `[UED]\Classes\Settings\EditorLoadingSavingSettings.h:205`) ou notification toast « Checkout ? » (si `bPromptForCheckoutOnAssetModification`, UnrealEdEngine.cpp:600) ; si `IsCheckedOutOrModifiedInOtherBranch` → warning (:707-709).

### 4.2 Save : « Prompt for checkout and save » et « Make Writable »

- `FEditorFileUtils::PromptForCheckoutAndSave` (`[UED]\Public\FileHelpers.h:507-509`) : flux standard de TOUTES les sauvegardes (Save All, save de niveau, build, etc.). Il vérifie l'état SCC des packages à sauver, propose le checkout des fichiers `CanCheckout()`, et pour les fichiers non-checkoutables (verrouillés par un autre, pas de connexion…) propose **« Make Writable »** (bouton `DRT_MakeWritable`, `Engine\Source\Editor\PackagesDialog\Public\PackagesDialog.h:18`, géré dans `SPackagesDialog.cpp:537` ; écriture forcée via `FEditorFileUtils::MakePackagesWritable`, FileHelpers.h:705).
- Le checkout silencieux au save est piloté par `GetAutomaticallyCheckoutOnAssetModification` (FileHelpers.cpp:477).
- Un fichier rendu inscriptible sans checkout part dans les **Uncontrolled Changelists** (voir 4.7).

### 4.3 Fenêtre Submit (check-in)

- Entrées : `FSourceControlWindows::ChoosePackagesToCheckIn` (`[SCW]\Public\SourceControlWindows.h:98`) — bouton « Submit Content » du status bar / menu Revision Control — et `PromptForCheckin` (:135) qui affiche `SSourceControlSubmitWidget` (`[SCW]\Private\SSourceControlSubmit.cpp`).
- Contenu : liste des fichiers cochables (modifiés + à ajouter + à supprimer), champ description obligatoire, case **« Keep Files Checked Out »** (SSourceControlSubmit.cpp:461-478, transmise via `FCheckIn::SetKeepCheckedOut`), panneau de **validation de changelist** (résultat/warnings/erreurs affichés, :365-367 ; hooks de pré-submit type Data Validation).
- Avant submit, le flux sauve les packages dirty (`PromptForCheckoutAndSave`, SourceControlWindows.cpp:714) puis exécute `MarkForAdd`+`Delete`+`CheckIn`. Résultat détaillé dans `FCheckinResultInfo` (SourceControlWindows.h:65-84).

### 4.4 View Changelists & Shelves

- Onglet « View Changelists » : `ISourceControlWindowsModule::ShowChangelistsTab` (visible si le provider répond `UsesChangelists()` ; ouvert depuis le menu du status bar, `Engine\Source\Editor\StatusBar\Private\SourceControlMenuHelpers.cpp:119,161`). Widget : `SSourceControlChangelists` (`[SCW]\Private\SSourceControlChangelists.cpp`).
- Fonctions : arbre CL → fichiers (+ shelves), créer/éditer/supprimer une CL (`NewChangelist`/`EditChangelist`/`DeleteChangelist`), drag & drop de fichiers entre CL (`MoveToChangelist`), revert unchanged, shelve/unshelve/delete shelved, submit d'une CL précise, diff d'un fichier shelvé (`FSourceControlWindows::DiffAgainstShelvedFile`, SourceControlWindows.h:209).

### 4.5 History, diff & diff visuel de Blueprints

- Menu contextuel d'asset → History : `FSourceControlWindows::DisplayRevisionHistory` (SourceControlWindows.h:176) → `UpdateStatus` avec `SetUpdateHistory(true)` (donc `p4 filelog`) → fenêtre `SSourceControlHistory` (`[SCW]\Private\SSourceControlHistory.cpp`) : liste des révisions (n°, CL, auteur, date, description), diff entre deux révisions sélectionnées, contexte menu par révision.
- **Diff d'assets binaires (.uasset)** : `UAssetToolsImpl::DiffAgainstDepot` (`Engine\Source\Developer\AssetTools\Private\AssetTools.cpp:3230`, API `IAssetTools.h:606`) télécharge la révision (via `ISourceControlRevision::Get()` → temp file), charge les deux packages et appelle `DiffAssets` (IAssetTools.h:610) qui ouvre **l'outil de diff spécifique à la classe d'asset** : Blueprint Diff Tool (graphes côte à côte), diff de DataTable, de Material, etc. Le provider n'a besoin QUE de fournir « le contenu du fichier à la révision N » — tout le diff visuel est fait par l'éditeur.
- Diff vs workspace : `FSourceControlWindows::DiffAgainstWorkspace` (SourceControlWindows.h:200).

### 4.6 Status bar, Sync, Revert, conflits

- Status bar (en bas à droite) : `SourceControlMenuHelpers.cpp` — indicateur de connexion (:355-381 : « Contacting Server… », « Server Unavailable »…), menu Actions (:279-339) : Connect/Change settings, View Changelists, Submit Content, Check Out Modified Files, Sync Latest, Revert All.
- `FSourceControlWindows::SyncLatest`/`SyncRevision` (SourceControlWindows.h:112,119) : sauvegarde tout, `FSync`, puis **reload du monde** et des assets touchés.
- Revert : `PromptForRevert` (SourceControlWindows.h:186, dialog `SSourceControlRevert.cpp` avec option « revert unchanged only ») et `RevertAllChangesAndReloadWorld` (:191, confirmation SourceControlMenuHelpers.cpp:228-229).
- Conflits : un fichier `Conflicted` (unresolved après sync) est badgé ; la résolution offerte est `FResolve` = **accept yours** (garder local) ou re-sync forcé (accepter le serveur). Il n'y a PAS de merge 3-way d'assets binaires dans l'éditeur — la « résolution » Perforce niveau artiste est un choix mine/theirs.
- Menu contextuel Content Browser (assets et fichiers) : Check Out, Make Writable, Check In, Revert, History, Diff, Refresh, Copy File Path — ex. implémentation fichiers : `Engine\Plugins\Editor\ContentBrowser\ContentBrowserFileDataSource\...\FileSourceControlContextMenu.cpp:121-137` (avec désactivation dynamique selon `CanExecuteSCC*` et affichage du/des utilisateurs qui ont le fichier, :385).

### 4.7 Uncontrolled Changelists (« travailler sans se connecter »)

Module dédié : `Engine\Source\Developer\UncontrolledChangelists` (`UncontrolledChangelistsModule.h`). Activé si le provider répond `UsesUncontrolledChangelists()`.
- Tout fichier rendu inscriptible sans checkout (`OnMakeWritable`, :75), sauvé sans provider (`OnSaveWritable`, :82), supprimé (`OnDeleteWritable`, :89) ou ajouté hors ligne (`OnNewFilesAdded`, :96) est tracé dans des changelists **locales**, persistées en JSON (`SaveState`/`LoadState`, :265-270).
- Bouton **« Reconcile assets »** (`OnReconcileAssets`, :147) : re-scanne les assets précédemment notés pour détecter les modifications faites hors connexion.
- Une fois reconnecté, `MoveFilesToControlledChangelist` (:189) fait le vrai checkout des fichiers (avec dialog de conflit si quelqu'un d'autre les a pris entre-temps).
- Hook de save : `OnObjectPreSaved` (:166). CRUD de CL uncontrolled : :205-218.
- L'UI vit dans la même fenêtre View Changelists (section « Uncontrolled »).

### 4.8 One File Per Actor (OFPA) / World Partition

Pas de logique Perforce spécifique dans le provider : OFPA passe par les mêmes flux (les acteurs externes `__ExternalActors__/…` sont des packages comme les autres, sauvés en masse via `PromptForCheckoutAndSave`, cf. `WorldPartitionEditorModule.cpp:1101-1105`). Conséquence pratique pour un provider : **des `UpdateStatus`/`CheckOut`/`CheckIn` sur des MILLIERS de petits fichiers** — la performance en batch et le cache d'états sont critiques, plus que toute feature.

### 4.9 Réglages de connexion

Dialog générique `SSourceControlLogin` + widget provider `SPerforceSourceControlSettings.cpp` (`[P4]`) : Port/User/Workspace (+ auto-détection), persistés dans `Saved/Config/WindowsEditor/SourceControlSettings.ini` (cf. le fichier du projet FirstPerson). Le plugin Lore fournira son propre widget (chemin du repo, identité, état du service).

---

## 5. Sémantique des verrous (exclusive checkout)

Perforce a DEUX notions que l'UI d'UE agrège :
1. **Open for edit** (`p4 edit`) : plusieurs personnes peuvent ouvrir le même fichier en même temps ; l'éditeur montre alors « also checked out by X » (`otherOpen`), et le submit du second fera un resolve.
2. **Verrou exclusif** : le filetype `+l` (posé sur les `.uasset`/`.umap` via typemap serveur) rend le checkout EXCLUSIF — `p4 edit` échoue si quelqu'un d'autre l'a déjà. Détection dans le provider : `headType` contient `+l` → `bExclusiveCheckout` (PerforceSourceControlOperations.cpp:1512-1514, :1644). C'est LE mode de travail standard des assets binaires UE.

Côté UI : `IsCheckedOutOther(&Who)` remonte le(s) nom(s) (`OtherUserCheckedOut`, PerforceSourceControlState.h:135) ; l'icône `CheckedOutByOtherUser` porte un badge, le tooltip dit « Checked out by: X », les entrées de menu Check Out sont désactivées, le prompt de save propose seulement « Make Writable » (à ses risques). Le worker de checkout re-vérifie et l'erreur remonte en notification (« checked out by X », usage `IsCheckedOutOther` p.ex. PerforceSourceControlOperations.cpp:2160).

**Équivalence Lore** : `lore lock` est un système de verrous exclusifs par branche (`acquire`/`release`/`status`/`query --branch/--owner/--path`). C'est exactement la sémantique `+l`. Le modèle Lore est même plus simple : pas de mode « open non-exclusif » à émuler — le plugin peut poser lock = checkout, release = revert/submit.

---

## 6. Surface CLI `lore` constatée (vérifiée par `--help`)

Commandes pertinentes pour le plugin (top-level) : `status [--scan|--check-dirty|--count]`, `dirty`, `stage [--scan]` (+ `stage move`, `stage merge`), `unstage`, `reset [--purge] [--revision]`, `diff [--source|--target|--diff3]`, `history`, `commit <msg>`, `push [--fast-forward-merge]`, `sync [rev] [--forward-changes|--reset|--root-file|--dependency-*]`, `lock acquire|release|status|query`, `clone`, `login`/`auth`, `notification subscribe`, `service`, `branch list|info|create|switch|merge|latest|…`, `revision history|info|diff|revert|cherry-pick|…`, `file info [--revision]|history|diff|write|metadata|dependency|hash`, `layer`, `link`, `logfile`, `shared-store`.

Points notables découverts pendant l'audit :
- **`lore file write --path <PATH> --revision <REV> --output <OUT>` existe** : c'est l'équivalent de `p4 print` (contenu d'un fichier, y compris binaire, à une révision arbitraire, sans toucher l'état local). Le manque supposé « pas de `lore file cat <rev>` » (noté en mémoire de roadmap) est en réalité **couvert** — à confirmer sur un binaire, mais la surface CLI y est. Débloque : History+Diff visuel de Blueprints, DownloadFile, GetFile.
- `lore status --scan` persiste les dirty flags : le plugin peut faire un scan initial puis des updates ciblés (`dirty` + `status --check-dirty`), ce qui colle au modèle « UpdateStatus sur les packages dirtied this tick » d'UE.
- `lore notification subscribe` : push temps réel des événements du repo — mieux que le polling `fstat` de Perforce pour tenir à jour overlays et « locked by X ».
- `--dry-run` global : donne un `SyncPreview` gratuit.
- `.loreignore` natif : mappe l'état `Ignored`.
- `lore sync <rev> --reset/--forward-changes` + sync sélectif par dépendances (`--root-file`, `--dependency-*`) : au-delà de la parité Perforce.

---

## 7. Mapping capacité UE ⇄ CLI Lore

Verdicts : ✅ couvrable tel quel · ⚠️ couvrable avec contrainte · ❌ bloqué (manque CLI).

| Capacité UE (opération/flux) | Verdict | Mapping CLI lore & contraintes |
|---|---|---|
| `Connect` / login dialog / statut serveur | ✅ | `lore login` / `lore auth`, `lore repository` + `lore status --revision-only` pour sonder ; `--offline` pour l'état dégradé. Widget de settings à écrire (chemin repo, identité). |
| `UpdateStatus` (états fichiers, qui a quoi) | ✅ | `lore status [--scan|--check-dirty]` pour modifié/ajouté/supprimé + `lore lock status/query` pour checked-out-by (owner). Sortie parsable à confirmer (JSON souhaitable) ; sinon parsing texte. Perf sur gros arbres : scan initial + dirty ciblé + notifications. |
| « Not at head revision » | ⚠️ | Comparer révision synchronisée (`lore status`) à `lore branch latest` / `lore revision info` ; par fichier : `lore file history <path> 1` vs état local. Faisable mais c'est au plugin d'assembler — pas de champ « haveRev/headRev » clé en main par fichier. |
| `CheckOut` (exclusif, `+l`) | ✅ | `lore lock acquire <paths>` (+ rendre le fichier inscriptible côté plugin). Sémantique lock par branche = exclusive checkout. « Checked out by X » = `lock query --path` → owner. |
| Read-only local (`UsesLocalReadOnlyState`) | ⚠️ | Lore ne gère pas le bit read-only des fichiers (pas de `p4 sync` qui pose +r). Le plugin (ou le client desktop) doit poser/enlever lui-même l'attribut lecture seule s'il veut le workflow « fichier verrouillé tant que pas checked out ». Alternative : répondre `UsesLocalReadOnlyState=false` et s'appuyer sur le prompt de checkout à l'édition. |
| `MarkForAdd` | ✅ | `lore dirty <path>` puis `stage` (ou `stage <file>` direct). |
| `Delete` | ✅ | suppression disque + `lore stage` (le scan détecte les deletes) ; `lore stage move` pour les renames — mieux que `p4 delete`+`add`. |
| `Revert` (fichier / all / delete-new-files) | ✅ | `lore unstage` + `lore reset <paths>` ; `--purge` = supprimer les nouveaux fichiers (équivalent `ShouldDeleteNewFilesOnRevert`) ; revert all = `lore reset .` + `unstage`. |
| `RevertUnchanged` | ✅ | `lore status --check-dirty` fait exactement ça (déflague les fichiers identiques au depot). |
| `Sync` (latest / révision) | ✅ | `lore sync [rev]` ; `--reset` (force), `--forward-changes`. Reload monde/assets = côté plugin (fourni par le framework UE). |
| `SyncPreview` | ✅ | `lore sync <rev> --dry-run`. |
| `CheckIn` (description, keep checked out) | ⚠️ | `lore stage` → `lore commit "<desc>"` → `lore push [--fast-forward-merge]`. Contraintes : (1) commit+push = 2 étapes, gérer l'échec de push (head avancée) → message clair + retry fast-forward ; (2) « Keep checked out » = ne pas relâcher les locks après push (trivial) ; (3) le succès doit rapporter le hash de révision (parser la sortie). |
| Fenêtre Submit + validation | ✅ | UI fournie par UE (`SSourceControlSubmitWidget`) dès que le provider répond aux opérations ; la validation (Data Validation) est côté éditeur. |
| `History` par asset | ✅ | `lore file history <path> [N]` (auteur, date, message, révision). |
| Diff visuel Blueprint / diff vs depot | ✅ | `lore file write --path X --revision R --output tmp.uasset` puis `IAssetTools::DiffAssets` — tout le diff visuel est déjà dans l'éditeur. À valider sur binaire, mais la commande existe. |
| `DownloadFile` / `GetFile` | ✅ | idem `lore file write` (+ `--address` pour un blob direct). |
| `GetSourceControlRevisionInfo` | ✅ | `lore revision info <rev>`. |
| `Where` (mapping chemins) | ✅ | trivial : chemins relatifs au root du repo (pas de clientspec/vue chez Lore). |
| `GetFileList` | ⚠️ | `lore file info <dir>` / `lore status --count` ; pas de listing récursif « du serveur » documenté avec suppression incluse (`--include-deleted` ❌). Rarement utilisé par l'artiste. |
| `Copy` (Branch) | ⚠️ | Pas d'`integrate` fichier-à-fichier. MAIS le défaut UE 5.4+ est `ECopyMethod::Add` (simple add), que Lore couvre (`stage` du nouveau fichier). Verdict : couvrable en mode Add ; la parenté de branche par fichier est perdue (acceptable, comportement par défaut d'UE). |
| `Resolve` (conflits) | ⚠️ | Modèle Lore : `lore sync` sur du modifié → `--forward-changes`/`--reset` ; `lore stage merge` + `lore diff --diff3` pour le texte. Pour les binaires : accept mine = garder local + `stage`, accept theirs = `lore reset --revision <head>`. Sémantiquement couvert, mais l'état « Conflicted » n'est pas exposé tel quel par `status` — à assembler par le plugin (lock + retard + modifié). À confirmer selon le comportement exact de sync sur fichiers dirty. |
| Changelists (`NewChangelist`, `MoveToChangelist`, `EditChangelist`, `DeleteChangelist`, `GetPendingChangelists`, `GetChangelistDetails`, `GetSubmittedChangelists`) | ❌ (multi) / ⚠️ (mono) | Lore n'a qu'UNE staged revision par repo (pas de changelists nommées multiples). Mapping v1 : répondre `UsesChangelists()=false` → l'éditeur bascule sur le flux fichiers (comme le provider Git) et masque la fenêtre Changelists. La « CL par défaut » = staged state (`lore status`). `GetSubmittedChangelists` = `lore history`. Multi-CL : manque CLI réel (multi-staging), à traiter côté produit si demandé. |
| `Shelve` / `Unshelve` / `DeleteShelved` | ❌ | Aucun équivalent CLI (pas de stockage serveur de WIP hors commit). Contournement possible : commit sur branche perso (`lore branch create` + push) — mais ce n'est pas un mapping direct. Masqué automatiquement si `UsesChangelists()=false`. |
| Uncontrolled Changelists / travail hors ligne | ✅ | Force de Lore : `--offline` global, dirty flags persistants, `status --scan` = « Reconcile assets ». Le module UE fonctionne côté éditeur ; répondre `UsesUncontrolledChangelists()` selon la stratégie retenue (probablement false en v1 : Lore travaille offline nativement, commit local possible sans réseau). |
| `CreateWorkspace`/`DeleteWorkspace`/`GetWorkspaces` | ⚠️ | `lore clone` couvre l'onboarding ; pas de notion de clientspec à créer/supprimer. Hors quotidien artiste (utilisé par UGS/Horde). |
| Verrous : « checked out by X » temps réel | ✅ | `lore lock query` + `lore notification subscribe` (push) — potentiellement MEILLEUR que Perforce (pas de polling). |
| États/overlays (`Ignored`, `NotControlled`, `Added`…) | ✅ | `.loreignore` natif (Ignored) ; `status` distingue add/modify/delete ; NotControlled = hors repo/ignoré. Overlays = implémentation `ISourceControlState::GetIcon` par le plugin (style set `RevisionControl.*` réutilisable tel quel). |
| Checkout-on-edit / prompts de save / Make Writable | ✅ | Entièrement côté éditeur (UnrealEdEngine/FileHelpers) : le plugin n'a qu'à répondre correctement à `UpdateStatus`, `CanCheckout`, `CheckOut`. « Make Writable » → marquer `lore dirty` (miroir du flux Uncontrolled). |
| Multi-branches (« checked out in other branch ») | ⚠️ | `lore lock query --branch` et `lore branch diff` existent ; l'agrégation par fichier (`CheckedOutBranches`) est à assembler. Confort, pas bloquant v1. |

### Manques CLI identifiés (synthèse)

**Bloquants pour une parité complète (mais pas pour un artiste, cf. §8)** :
1. Changelists multiples nommées (multi-staging) — sans équivalent.
2. Shelve/unshelve (WIP serveur sans commit) — sans équivalent direct.

**Contournables / à confirmer** :
3. Sortie machine-readable (JSON) pour `status`, `lock query`, `file history` — non vérifiée dans les helps ; si absente, le plugin parsera du texte (fragile). Recommandation forte : ajouter `--json`.
4. Exposition directe par fichier de « haveRev vs headRev » et d'un état « conflicted » — assemblable côté plugin, coûteux sur de gros arbres ; un `lore status --against-latest` aiderait.
5. Gestion du bit read-only local — à faire côté plugin/service.
6. `lore file write` sur binaire à une révision : la commande existe, comportement à valider (le « manque `file cat` » de la roadmap est a priori résolu par `file write`).

---

## 8. Recommandation de périmètre v1

### Tier 1 — v1 indispensable (parité artiste quotidienne)
1. **Provider + connexion** : implémentation `ISourceControlProvider` (Connect, settings widget, statut serveur), capacités : `UsesCheckout=true`, `UsesChangelists=false`, `UsesLocalReadOnlyState` à trancher, `AllowsDiffAgainstDepot=true`, `UsesFileRevisions=true`.
2. **UpdateStatus + overlays** : status/lock query → tous les états (CheckedOut, CheckedOutOther+qui, Added, Deleted, Modified, NotAtHead, Ignored, NotControlled), alimenté par `notification subscribe` pour le temps réel.
3. **CheckOut = lock acquire** (avec message « locked by X » propre) + checkout-on-edit + prompt de save + Make Writable→dirty.
4. **MarkForAdd / Delete / Revert** (dirty/stage, reset [--purge], unstage) + RevertUnchanged (`status --check-dirty`).
5. **CheckIn** : fenêtre Submit standard → stage → commit → push (+ release des locks, option keep checked out, gestion d'échec de push).
6. **Sync Latest / Sync revision** (+ dry-run pour préviews) avec reload monde.
7. **History par asset + diff visuel** (file history + file write --revision → DiffAssets/Blueprint Diff).
8. **Perf OFPA** : batching massif des status/locks, cache d'états, scan initial puis incrémental.

### Tier 2 — v2 confort
- État « conflicted » de première classe et flux de résolution guidé (mine/theirs) ; multi-branches dans les tooltips (locks des autres branches) ; `GetSubmittedChangelists` (historique repo dans l'UI) ; travail hors ligne poli (`UsesUncontrolledChangelists` ou équivalent natif Lore) ; sync sélectif par dépendances (root-file) exposé ; sorties `--json` du CLI ; SyncPreview avec taille de transfert.

### Hors sujet (Perforce-spécifique, ne pas répliquer)
- Changelists multiples nommées et Shelve/Unshelve (modèle p4 ; l'équivalent Lore serait branches perso — décision produit séparée) ; Create/Delete/GetWorkspaces (clientspecs — remplacé par `lore clone`) ; `Copy` en mode Branch avec parenté (`p4 integrate`) ; `Where` (pas de vue client) ; `ChangeStatus` (interne p4) ; labels (`ISourceControlLabel`).

---

## 9. Annexe — fichiers de référence pour le développement

| Sujet | Fichier |
|---|---|
| Contrat provider | `[SC]\Public\ISourceControlProvider.h`, `ISourceControlState.h`, `ISourceControlOperation.h`, `ISourceControlRevision.h`, `ISourceControlChangelist.h` |
| Liste des opérations | `[SC]\Public\SourceControlOperations.h` |
| Exemple d'implémentation complet | `[P4]\PerforceSourceControlProvider.cpp`, `PerforceSourceControlOperations.cpp`, `PerforceSourceControlState.cpp` |
| Icônes/overlays | `[SC]\Public\RevisionControlStyle\RevisionControlStyle.h` |
| Fenêtres éditeur | `[SCW]\Private\SSourceControlSubmit.cpp`, `SSourceControlHistory.cpp`, `SSourceControlRevert.cpp`, `SSourceControlChangelists.cpp`, `SourceControlWindows.cpp` |
| Checkout auto / prompts | `[UED]\Private\UnrealEdEngine.cpp:572-720`, `[UED]\Private\FileHelpers.cpp`, `[UED]\Classes\Settings\EditorLoadingSavingSettings.h:205` |
| Offline | `Engine\Source\Developer\UncontrolledChangelists\Public\UncontrolledChangelistsModule.h` |
| Diff d'assets | `Engine\Source\Developer\AssetTools\Private\AssetTools.cpp:3230` (`DiffAgainstDepot`) |
| Status bar | `Engine\Source\Editor\StatusBar\Private\SourceControlMenuHelpers.cpp` |
| Menu contextuel fichiers | `Engine\Plugins\Editor\ContentBrowser\ContentBrowserFileDataSource\Source\ContentBrowserFileDataSource\Private\FileSourceControlContextMenu.cpp` |
