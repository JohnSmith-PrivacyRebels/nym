# Nym Operators Legal Forum: Running Exit Gateway

```admonish info
The entire content of this page is under [Creative Commons Attribution 4.0 International Public License](https://creativecommons.org/licenses/by/4.0/).
```

This page is a part of Nym Community Legal Forum and its content is composed by shared advices in [Node Operators Legal Forum](https://matrix.to/#/!YfoUFsJjsXbWmijbPG:nymtech.chat?via=nymtech.chat&via=matrix.org) (Matrix chat) as well as though pull requests done by the node operators directly to our [repository](https://github.com/nymtech/nym/tree/develop/documentation/operators/src), reviewed by Nym DevRels.

This document presents an initiative to further support Nym’s mission of allowing privacy for everyone everywhere. This would be achieved with the support of Nym node operators operating Gateways and opening these to any online service. Such setup needs a **clear policy**, one which will remain the **same for all operators** running Nym nodes. The [proposed **Exit policy**](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) is a combination of two existing safeguards: [Tor Null ‘deny’ list](https://tornull.org/) and [Tor reduced policy](https://tornull.org/tor-reduced-reduced-exit-policy.php).

All the technical changes on the side of Nym nodes - ***Project Smoosh*** - are described in the [FAQ section](../archive/faq/smoosh-faq.md).

```admonish warning
Nym core team cannot provide comprehensive legal advice across all jurisdictions. Knowledge and experience with the legalities are being built up with the help of our counsel and with you, the community of Nym node operators. We encourage Nym node operators to join the operator channels ([Element](https://matrix.to/#/#operators:nymtech.chat), [Discord](https://nymtech.net/go/discord), [Telegram](https://t.me/nymchan_help_chat)) to share best practices and experiences.
```


## Summary

* This document outlines a plan to change Nym Gateways from operating with an ‘allow’ to a ‘deny’ list to enable broader uptake and usage of the Nym Mixnet. It provides operators with an overview of the plan, pros and cons, legal as well as technical advice. 

* Nym is committed to ensuring privacy for all users, regardless of their location and for the broadest possible range of online services. In order to achieve this aim, the Nym Mixnet needs to increase its usability across a broad range of apps and services.

* Currently, Nym Gateway nodes only enable access to apps and services that are on an ‘allow’ list that is maintained by the core team. 

* To decentralise and enable privacy for a broader range of services, this initiative will have to transition from the current ‘allow’ list to a ‘deny’ list - [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt). In accordance with the [Tor Null 'deny' list](https://tornull.org/) and [Tor reduced policy](https://tornull.org/tor-reduced-reduced-exit-policy.php), which are two established safeguards.

* This will enhance the usage and appeal of Nym products for end users. As a result, increased usage will ultimately lead to higher revenues for Nym operators.

* Nym core team cannot provide operators with definitive answers regarding the potential risks of operating open Gateways. However, there is online evidence of operating Tor exit relays:
	* From a technical perspective, Nym node operators may need to implement additional controls, such as dedicated hardware and IP usage, or setting up an HTML exit notice on port 80.
	* From an operational standpoint, node operators may be expected to actively manage their relationship with their ISP or VPS provider and respond to abuse requests using the proposed templates.
	* Legally, exit relays are typically considered "telecommunication networks" and are subject to intermediary liability protection. However, there may be exceptions, particularly in cases involving criminal law and copyright claims. Operators could seek advice from local privacy associations and may consider running nodes under an entity rather than as individuals.

* This document serves as the basis for a consultation with Nym node operators on any concerns or additional support and information you need for this change to be successful and ensure maximum availability, usability and adoption.

## Goal of the initiative

**Nym supports privacy for everyone, everywhere.**

To offer a better and more private everyday experience for its users, Nym would like them to use any online services they please, without limiting its access to a few messaging apps or crypto wallets.

To achieve this, operators running Exit Gateways would have to “open” their nodes to a wider range of online services, in a similar fashion to Tor exit relays following this [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt).

## Pros and cons of the initiative

Previous setup: Running nodes supporting strict SOCKS5 app-based traffic

| **Dimension** | **Pros** | **Cons** |
| :--- | :--- | :--- |
| Aspirational |   | - Very limited use cases, not supportive of the “Privacy for everyone everywhere” aspiration<br>- Limited appeal to users, low competitiveness in the market, thus low usage |
| Technical | - No changes required in technical setup |   |
| Operational | - No impact on operators operations (e.g., relationships with VPS providers)<br>- Low overhead<br>- Can be run as an individual |   |
| Legal | - Limited legal risks for operators |    |
| Financial |    | - Low revenues for operators due to limited product traction | 


The new setup: Running nodes supporting traffic of any online service (with safeguards in the form of a denylist)

| **Dimension** | **Pros** | **Cons** |
| :--- | :--- | :--- |
| Aspirational | - Higher market appeal of a fully-fledged product able to answer all users’ use cases<br>- Relevance in the market, driving higher usage |   |
| Technical | - Very limited changes required in the technical setup (changes in the allow -> denylist) | - Increased monitoring required to detect and prevent abuse (e.g. spam) |
| Operational |    | - Higher operational overhead, such as dealing with DMCA / abuse complaints, managing the VPS provider questions, or helping the community to maintain the denylist <br>- Administrative overhead if running nodes as a company or an entity |
| Legal |   | - Ideally requires to check legal environment with local privacy association or lawyer | Financial | - Higher revenue potential for operators due to the increase in network usage | - If not running VPS with an unlimited bandwidth plan, higher costs due to higher network usage |

## Exit Gateways: New setup

In our previous technical setup, Network Requesters acted as a proxy, and only made requests that match an allow list. That was a default IP based list of allowed domains stored at Nym page in a centralised fashion possibly re-defined by any Network Requester operator. 

This restricts the hosts that the NymConnect app can connect to and has the effect of selectively supporting messaging services (e.g. Telegram, Matrix) or crypto wallets (e.g. Electrum or Monero). Operators of Network Requesters can have confidence that the infrastructure they run only connects to a limited set of public internet hosts.

The principal change in the new configuration is to make this short allow list more permissive. Nym's [exit policy](https://nymtech.net/.wellknown/network-requester/exit-policy.txt) will restrict the hosts to which Nym Mixnet and Nym VPN users are permitted to connect. This will be done in an effort to protect the operators, as Gateways will act both as SOCKS5 Network Requesters, and exit nodes for IP traffic from Nym Mixnet VPN and VPN clients (both wrapped in the same app). 

As of now we the Gateways will be defaulted to a combination of [Tor Null ‘deny’ list](https://tornull.org/) (note: Not affiliated with Tor) - reproduction permitted under Creative Commons Attribution 3.0 United States License which is IP-based, e.g., `ExitPolicy reject 5.188.10.0/23:*` and [Tor reduced policy](https://tornull.org/tor-reduced-reduced-exit-policy.php). Whether we will stick with this list, do modifications or compile another one is still a subject of discussion. In all cases, this policy will remain the same for all the nodes, without any option to modify it by Nym node operators to secure stable and reliable service for the end users.

For exit relays on ports 80 and 443, the Gateways will exhibit an HTML page resembling the one proposed by [Tor](https://gitlab.torproject.org/tpo/core/tor/-/raw/HEAD/contrib/operator-tools/tor-exit-notice.html). By doing so, the operator will be able to disclose details regarding their Gateway, including the currently configured exit policy, all without the need for direct correspondence with regulatory or law enforcement agencies. It also makes the behavior of Exit Gateways transparent and even computable (a possible feature would be to offer a machine readable form of the notice in JSON or YAML).

We also recommend operators to check the technical advice from [Tor](https://community.torproject.org/relay/setup/exit/).

## Tor legal advice

Giving the legal similarity between Nym Exit Gateways and Tor Exit Relays, it is helpful to have a look in [Tor community Exit Guidelines](https://community.torproject.org/relay/community-resources/tor-exit-guidelines/). This chapter is an exert of tor page.

Note that Tor states: 
> This FAQ is for informational purposes only and does not constitute legal advice.

*Check legal advice prior to running an exit relay*  

* Understand the risks associated with running an exit relay; E.g., know legal paragraphs relevant in the country of operations:
    - US [DMCA 512](https://www.law.cornell.edu/uscode/text/17/512); see [EFF's Legal FAQ for Tor Operators](https://community.torproject.org/relay/community-resources/eff-tor-legal-faq) (a very good and relevant read for other countries as well)
    - Germany’s [TeleMedienGesetz 8](http://www.gesetze-im-internet.de/tmg/__8.html) and [15](http://www.gesetze-im-internet.de/tmg/__15.html)
    - Netherlands: [Artikel 6:196c BW](http://wetten.overheid.nl/BWBR0005289/Boek6/Titel3/Afdeling4A/Artikel196c/)
    - Austria: [E-Commerce-Gesetz 13](http://www.ris.bka.gv.at/Dokument.wxe?Abfrage=Bundesnormen&Dokumentnummer=NOR40025809)
    - Sweden: [16-19 2002:562](https://lagen.nu/2002:562#P16S1)
* Top 3 advice
    - Have an abuse response letter
    - Run relay from a location that is not home
    - Read through the legal resources that Tor-supportive lawyers put together: [www.eff.org/pages/legal-faq-tor-relay-operators](https://www.eff.org/pages/legal-faq-tor-relay-operators) or [www.noisebridge.net/wiki/Noisebridge_Tor/FBI](https://www.noisebridge.net/wiki/Noisebridge_Tor/FBI)
* Consult a lawyer / local digital rights association / the EFF prior to operating an exit relay, especially in a place where exit relay operators have been harassed or not operating before. Note that Tor DOES NOT provide legal advice for specific countries. It only provides general advice (itself or in partnership), eventually skewed towards [US audiences](https://www.eff.org/pages/legal-faq-tor-relay-operators).


*Run an exit relay within an entity*  

As an organisation - it might help from a liability perspective  
* Within your university
* With a node operators’ association (e.g., a Torservers.net partner)
* Within a company

*Be ready to respond to abuse complaints*

* Make your contact details (email, phone, or even fax) available, instead of those of the ISP
* Reply in a timely manner (e.g., 24 hours) using the [provided templates](https://community.torproject.org/relay/community-resources/tor-abuse-templates)
* Note that Tor states: *“We are not aware of any case that made it near a court, and we will do everything in our power to support you if it does.”*
* Document experience with ISPs at [community.torproject.org/relay/community-resources/good-bad-isps](https://community.torproject.org/relay/community-resources/good-bad-isps/)

Useful links:  

* Tor abuse templates:
    - [community.torproject.org/relay/community-resources/tor-abuse-templates/](https://community.torproject.org/relay/community-resources/tor-abuse-templates/)
    - [gitlab.torproject.org/legacy/trac/-/wikis/doc/TorAbuseTemplates](https://gitlab.torproject.org/legacy/trac/-/wikis/doc/TorAbuseTemplates) (from 2020)
    - [github.com/coldhakca/abuse-templates/blob/master/generic.template](https://github.com/coldhakca/abuse-templates/blob/master/generic.template)

* DMCA response templates:
    - [community.torproject.org/relay/community-resources/eff-tor-legal-faq/tor-dmca-response/](https://community.torproject.org/relay/community-resources/eff-tor-legal-faq/tor-dmca-response/)
    - [2019.www.torproject.org/eff/tor-dmca-response.html](https://2019.www.torproject.org/eff/tor-dmca-response.html) (from 2011)
    - [github.com/coldhakca/abuse-templates/blob/master/dmca.template](https://github.com/coldhakca/abuse-templates/blob/master/dmca.template)


## Legal environment of Nym Exit Gateway

The Node Operators Legal Forum pages are divided according [jurisdictions](jurisdictions.md). Read below to learn [how to add](add-content.md) findings about your jurisdiction or edit existing info.

## Community Counsel

Nym Node operators are invited to add their legal findings or helpful suggestions directly through [pull requests](add-content.md). This can be done as a new legal information (or entire new country) to the list of [jurisdictions](jurisdictions.md) or in a form of an advice to [Community counsel pages](community-counsel.md), sharing examples of Exit Gateway [landing pages](landing-pages.md), templates etcetra.

## How to add content

Our aim is to establish a strong community network, sharing legal findings and other suggestions with each other. We would like to encourage all of the current and future operators to do research about the situation in the jurisdiction they operate in as well as solutions to any challenges when running an Exit Gateway and add those through a pull request (PR). Please check out the [steps to make a pull request](add-content.md).
