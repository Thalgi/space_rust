# Méthode — la discipline du projet

> Ce qui a marché, et qu'il ne faut pas perdre. **À lire au démarrage d'une
> session**, avant d'écrire un test.
>
> Extrait de [`../STATE.md`](../STATE.md) le 2026-08-10, où il était noyé sous
> mille huit cents lignes de journal.

---


- **Red-check systématique** : tout test ajouté est cassé volontairement sur sa
  cible pour vérifier qu'il rougit. **Sauvegarder le fichier dans le scratchpad
  avant de le saboter**, et le restaurer par copie — jamais par git.
  Trois formes du même piège reviennent : un test qui **inspecte l'état** au
  lieu d'emprunter le vrai chemin de code, qui **fixe une variable** du scénario
  au lieu de la sonder (`libres()[0]`), ou qui se compare à **sa propre
  constante** (§F.7). La parade est toujours la même : muscler le test dès qu'un
  sabotage ne mord pas, jamais l'admettre « suffisamment proche ».
- ⚠️ **Un test rouge ne veut pas dire que le code a tort** (L2.4).
- ⚠️ **Un proxy qui ne tient que dans un régime n'est pas une mesure** (§F.2).
- ⚠️ **Une propriété d'accord entre deux appelants d'un même calcul ne garde pas
  ce calcul — elle garde son unicité.** Il faut un **second** test qui pinne la
  valeur.
- **Une source par fait.** Presque toutes les erreurs de ce projet viennent
  d'une valeur qui en a deux (`docs/suivi/stations.md` §C.29).
- **Une cote qui se règle contre une autre pièce ne tombe pas sur `Profil`** —
  vu quatre fois. Réponse : mise à l'échelle **géométrique**, ou rester sur la
  grille et s'arrêter au cran qui tient (§F.4).
- **Brique d'abord, assemblage ensuite.** Jugée seule avant d'être posée.
- ⚠️ **La tolérance d'un test doit être plus petite que l'erreur qu'il cherche.**
  Vu le 2026-08-06 : le placement d'un hublot était borné par l'écart tangentiel
  de la paire (0,62) alors que l'erreur à détecter valait 0,14 — poser le vitrage
  au mauvais rayon passait au vert. La parade : mesurer sur un axe où le bruit
  n'existe pas (ici, la projection sur l'azimut), pas élargir la borne.
- ⚠️ **Comparer une copie à une copie ne prouve rien.** Quatre fois dans la même
  session : masses recopiées dans le test, corde *supposée* axiale au lieu d'être
  lue, ventre auto-référent (bardage comparé aux volets, jamais à une référence),
  commentaire de `braquage` que rien ne vérifiait. Toujours comparer à **la
  source**.
- ⚠️ **`dead_code` ne sait pas ce qu'est un test.** Il compte sur la cible
  binaire : tout ce que seuls les tests appellent y ressort mort. Mesuré le
  2026-08-05 — 90 morts pour le binaire, **13** pour les deux cibles. Conclure
  sans faire la différence mène à supprimer du code testé et qui marche.
- ⚠️ **Un sabotage qui ne rougit pas ne prouve pas que la garde est faible** —
  encore faut-il qu'il produise *vraiment* le défaut visé. Vu le 2026-08-10 :
  remonter la charge polaire de 14 unités lui faisait bien franchir le plan du
  miroir, mais en gardant 1,3 de dégagement — la garde avait raison de rester
  verte. Refait avec la bonne valeur, elle rougit. Avant de muscler un test,
  vérifier que le sabotage était le bon.
- ⚠️ **Un chiffre qui décide d'une pose ne doit pas vivre uniquement dans un
  test.** Vu le 2026-08-10 : le dégagement minimal entre trois modules d'habitat
  était calculé par un test de l'ISV, et le montage polaire — cinq fois plus fin
  — passait dessous sans que rien ne le voie. Le calcul **décide** désormais
  (§10.9), le test se contente de le vérifier.
- ⚠️ **Ajouter de la structure pour « expliquer » une pièce la rend souvent moins
  lisible, pas plus.** Trois fois : un mât au centre du miroir, une âme et des
  colliers dans la grappe de cuves, des cerclages par rangée. À chaque fois,
  retirer a mieux marché qu'ajouter. Corollaire : **deux pièces de la même
  famille doivent se peindre pareil**, sinon leur parenté ne se voit pas (la
  grappe était à la subdivision 2 quand le réservoir unitaire était à 3).
- **Mesurer avant d'affirmer.** Deux défauts de la session n'ont été trouvés que
  par une sonde qui imprime des positions : six pièces du pilier entièrement
  enfouies, et la palette qui filtrait par catégorie au lieu de filtrer par port.
  Aucun des deux n'était visible en relisant le code.
- **Mettre à jour les `.md`** à chaque étape — consigne debout de l'utilisateur.

