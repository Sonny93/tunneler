# Tunnels lancés hors de Tunneler

Vous avez peut-être déjà un `ssh -L 3333:...` qui tourne dans un terminal ou un script. Tunneler repère ces tunnels pour ne pas se battre avec eux pour le port.

## Comment ils sont détectés

Tunneler regarde qui écoute sur le port local de chacun de vos tunnels :

- quand vous démarrez un tunnel ;
- quand la fenêtre s'ouvre ;
- toutes les quelques secondes tant que la fenêtre est ouverte.

Il liste aussi les processus `ssh` qui redirigent des ports et qui ne correspondent à aucun de vos tunnels. Sans fenêtre ouverte, il n'y a pas de scan périodique : `tunneler status` montre le résultat du dernier.

## Un port tenu par un `ssh` qui fait la même redirection

Quand le processus sur le port est un `ssh` dont le `-L`, `-R` ou `-D` correspond au tunnel (même type, même port, adresse compatible, même cible), le tunnel est dans l'état « Externe » avec « fourni par » et le processus. Tunneler le considère comme déjà en marche et n'en démarre pas un second. L'hôte de destination de ce `ssh` n'est pas comparé.

Trois actions sont proposées :

- **Adopter.** Arrête ce processus et démarre, comme tunnels de Tunneler, tous les tunnels qu'il fournissait. Utilisez-le pour placer une redirection sous la responsabilité de Tunneler.
- **Arrêter le processus externe…** Arrête le processus `ssh`, après confirmation. Tous les tunnels que ce processus fournit s'arrêtent aussi.
- **Ignorer.** Cesse de signaler ce processus jusqu'à ce qu'il se termine. Ce choix n'est pas enregistré.

`tunneler up` compte un tunnel fourni par un `ssh` externe comme démarré.

## Un port tenu par autre chose

Quand un autre programme tient le port, le tunnel est dans l'état « Conflit » avec « port occupé par » le programme et son pid, par exemple `node.exe (pid 4242)`. Le tunnel ne démarre pas. « Arrêter le processus… » arrête ce programme après confirmation, et le tunnel peut ensuite être démarré.

Sous Linux, les sockets d'un autre utilisateur n'indiquent pas de pid sans être root : le propriétaire s'affiche alors « processus inconnu ».

## Les tunnels que Tunneler ne connaît pas

Les redirections `ssh` qui ne correspondent à aucun tunnel configuré apparaissent dans le panneau latéral sous « Externes non gérés ». Pour chacune :

- **Importer comme tunnel** ouvre le formulaire de nouveau tunnel rempli d'après le processus. Enregistrez-le et c'est un tunnel normal.
- **Arrêter le processus…** et **Ignorer** fonctionnent comme ci-dessus.

Un processus est identifié par son pid et son heure de démarrage : Tunneler n'arrête donc jamais un autre programme qui aurait réutilisé le pid. Tunneler exclut son propre processus du scan.

Il n'y a pas d'`adopt`, de `kill` ni d'`ignore` en ligne de commande.
