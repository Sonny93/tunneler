---
layout: home

hero:
  name: Tunneler
  text: Gestionnaire de tunnels SSH
  tagline: Un client SSH embarqué, un daemon dans la zone de notification et une interface lancée à la demande. Windows et Linux.
  image:
    src: /logo.svg
    alt: Logo de Tunneler
  actions:
    - theme: brand
      text: Prise en main
      link: /fr/guide/getting-started
    - theme: alt
      text: Feuille de route
      link: /fr/roadmap/
    - theme: alt
      text: GitHub
      link: https://github.com/Sonny93/tunneler

features:
  - title: Une session SSH par hôte
    details: Le client SSH est intégré. Activer ou couper un tunnel ouvre ou ferme un canal, sans se reconnecter. Il lit votre ~/.ssh/config et votre known_hosts.
  - title: Tags
    details: Donnez des tags à vos tunnels, par exemple dev ou mail, puis démarrez ou arrêtez tout un tag en un clic ou une commande.
  - title: Veille
    details: Un tunnel en veille garde son port local et n'ouvre la session SSH qu'à la première connexion.
  - title: Tunnels lancés ailleurs
    details: Les tunnels lancés avec ssh en dehors de Tunneler sont détectés. Vous pouvez les adopter, les tuer ou les importer.
  - title: Ouverture en un clic
    details: Ouvrez un tunnel dans le navigateur, ou dans une application comme un client VNC ou RDP choisie d'après le port cible.
  - title: Accessible
    details: L'interface se commande au clavier seul et se lit avec un lecteur d'écran. La CLI fonctionne sans couleur et hors terminal.
---
