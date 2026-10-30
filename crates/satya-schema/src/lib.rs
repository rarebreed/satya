//! # `satya-schema`
//!
//! Apache Avro Schema Registry (`CRC-64-AVRO`), 15-byte binary wire framing,
//! nested `Participant` identity, schema-driven GDPR PII anonymization & DSAR export,
//! and multi-modal research telemetry schemas for the **Satya** WebRTC & TTRPG platform.
//!
//! ## Module Structure
//! - [`types`]: Shared core identity and envelope types (`SessionMode`, `ParticipantKind`, `SessionRole`, `AiAgentInfo`, `Participant`, `EventMetadata`).
//! - [`events`]: Apache Iggy topic/subject constants and Avro domain event structs (`SchemaRegistryEvent`, `SessionLifecycleEvent`, `WebRtcTelemetryEvent`, `ChatMessageEvent`, `ChatMessageEventV2`, `InteractionDynamicsEvent`, `TtrpgMechanicsEvent`).
//! - [`protocol`]: REST API request/response payloads and WebSocket signaling envelopes (`ClientWsPacket`, `ServerWsPacket`, `StorageMetricsViewResponse`).
//! - [`registry`]: 15-byte binary wire framing (`WireFrameHeader`), `CRC-64-AVRO` fingerprinting, and `SchemaRegistry` encode/decode with schema evolution.
//! - [`gdpr`]: Schema-driven GDPR Art. 17 right-to-erasure anonymization and Art. 15/20 DSAR subject export powered by custom `satya.*` `.avsc` field attributes.

#![deny(missing_docs)]

pub mod events;
pub mod gdpr;
pub mod protocol;
pub mod registry;
pub mod types;

pub use events::*;
pub use gdpr::*;
pub use protocol::*;
pub use registry::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use apache_avro::Schema as AvroSchema;
    use uuid::Uuid;

    fn strip_satya_custom_props(val: &mut serde_json::Value) {
        match val {
            serde_json::Value::Object(map) => {
                map.retain(|k, _| !k.starts_with("satya."));
                for v in map.values_mut() {
                    strip_satya_custom_props(v);
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr.iter_mut() {
                    strip_satya_custom_props(item);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn test_custom_pii_annotations_preserve_crc64_avro_fingerprints_and_manifest() {
        let registry = SchemaRegistry::new_with_builtin_schemas().unwrap();
        for reg in registry.all_schemas() {
            // 1. Verify stripping all "satya.*" custom properties produces the exact same CRC-64-AVRO fingerprint
            let mut stripped_json: serde_json::Value = serde_json::from_str(&reg.raw_json).unwrap();
            strip_satya_custom_props(&mut stripped_json);
            let stripped_schema =
                AvroSchema::parse_str(&serde_json::to_string(&stripped_json).unwrap()).unwrap();
            let stripped_fp = crc64_avro_fingerprint(stripped_schema.canonical_form().as_bytes());
            assert_eq!(
                reg.fingerprint_crc64, stripped_fp,
                "CRC-64-AVRO fingerprint must be unaffected by satya.* custom attributes for {}",
                reg.subject
            );

            // 2. Verify every schema exposes its PII field manifest including metadata.timezone_offset_min and Participant.user_id
            let manifest = reg.extract_pii_field_manifest();
            assert!(
                manifest
                    .iter()
                    .any(|f| f.field_path == "metadata.timezone_offset_min"
                        && f.pii_owner.as_deref() == Some("PRIMARY_ACTOR")),
                "Missing metadata.timezone_offset_min in {}",
                reg.subject
            );
            assert!(
                manifest.iter().any(|f| f.is_gdpr_subject_key
                    && f.field_path.ends_with(".user_id")
                    && f.erasure_action.as_deref() == Some("HMAC_TOMBSTONE")),
                "Missing gdpr_subject_key user_id in {}",
                reg.subject
            );
        }
    }

    #[test]
    fn test_gdpr_primary_actor_vs_secondary_participant_anonymization_and_dsar() {
        let registry = SchemaRegistry::default();
        let sara = Participant::human("u-sara", "sara_v", "Sara (Valkyrie)", SessionRole::Player);
        let sean = Participant::human("u-sean", "gm_sean", "GM Sean", SessionRole::Gm);
        let ai_gm = Participant::ai_gm("sector-4", "qwen2.5:14b", "satya-archivist");

        let meta = registry
            .build_metadata(
                42,
                subjects::CHAT_MESSAGE,
                2,
                "sector-4",
                SessionMode::Ttrpg,
            )
            .unwrap();

        let msg = ChatMessageEventV2 {
            metadata: meta,
            message_id: "msg-42".to_string(),
            sender: sara.clone(),
            visibility: MessageVisibility::Public,
            recipients: vec![sean.clone(), ai_gm.clone()],
            channel_context: ChannelContext::InCharacter,
            reply_to_message_id: None,
            body: "Hey gm_sean, I open the airlock!".to_string(),
            char_count: 32,
            word_count: 6,
            composition_time_ms: 1500,
            is_edited: false,
            sentiment_score: Some(0.75),
            detected_intent: Some("ACTION_DECLARATION".to_string()),
        };

        let framed = registry
            .encode_framed(subjects::CHAT_MESSAGE, 2, &msg)
            .unwrap();

        // Case A: Secondary participant ("u-sean") requests Art. 17 erasure
        // Only recipients[0] should be anonymized and "gm_sean" scrubbed from body; Sara's sender & sentiment stay intact!
        let (match_sec, anon_sec_bytes) = registry
            .anonymize_framed_for_subject(&framed, "u-sean", "anon-sean999")
            .unwrap();
        assert_eq!(match_sec, GdprSubjectMatchKind::SecondaryParticipantOnly);
        let (_, decoded_sec): (_, ChatMessageEventV2) =
            registry.decode_framed(&anon_sec_bytes, None).unwrap();
        assert_eq!(decoded_sec.sender.user_id, "u-sara");
        assert_eq!(decoded_sec.recipients[0].user_id, "anon-sean999");
        assert_eq!(decoded_sec.recipients[0].username, "deleted_user");
        assert_eq!(
            decoded_sec.recipients[0].display_name,
            "Deleted Participant"
        );
        assert_eq!(decoded_sec.body, "Hey anon-sean999, I open the airlock!");
        assert_eq!(decoded_sec.sentiment_score, Some(0.75));

        // Case B: Primary actor ("u-sara") requests Art. 17 erasure
        // Sara's sender record is anonymized AND her primary-actor-owned fields (body, sentiment_score, detected_intent, timezone_offset_min) are redacted/nulled/zeroed!
        let (match_pri, anon_pri_bytes) = registry
            .anonymize_framed_for_subject(&framed, "u-sara", "anon-sara111")
            .unwrap();
        assert_eq!(match_pri, GdprSubjectMatchKind::PrimaryActor);
        let (_, decoded_pri): (_, ChatMessageEventV2) =
            registry.decode_framed(&anon_pri_bytes, None).unwrap();
        assert_eq!(decoded_pri.sender.user_id, "anon-sara111");
        assert_eq!(decoded_pri.sender.username, "deleted_user");
        assert_eq!(decoded_pri.body, "[Redacted per user erasure request]");
        assert_eq!(decoded_pri.sentiment_score, None);
        assert_eq!(decoded_pri.detected_intent, None);
        assert_eq!(decoded_pri.metadata.timezone_offset_min, 0);
        // Recipient Sean is untouched when Sara is erased
        assert_eq!(decoded_pri.recipients[0].user_id, "u-sean");

        // Case C: DSAR export for Sara ("u-sara") masks third-party human Sean's user_id/username, while preserving AI GM
        let (dsar_kind, dsar_msg): (_, ChatMessageEventV2) = registry
            .export_dsar_framed_for_subject(&framed, "u-sara")
            .unwrap()
            .unwrap();
        assert_eq!(dsar_kind, GdprSubjectMatchKind::PrimaryActor);
        assert_eq!(dsar_msg.sender.user_id, "u-sara");
        assert_eq!(dsar_msg.sender.username, "sara_v");
        assert_eq!(dsar_msg.recipients[0].user_id, "redacted-peer");
        assert_eq!(dsar_msg.recipients[0].username, "redacted_user");
        assert_eq!(dsar_msg.recipients[1].role, SessionRole::AiGm);
    }

    #[test]
    fn test_all_builtin_schemas_register_and_fingerprint() {
        let registry = SchemaRegistry::new_with_builtin_schemas().unwrap();
        let all = registry.all_schemas();
        assert_eq!(all.len(), 7);

        let chat_v1 = registry
            .get_by_subject_version(subjects::CHAT_MESSAGE, 1)
            .unwrap();
        let chat_v2 = registry
            .get_by_subject_version(subjects::CHAT_MESSAGE, 2)
            .unwrap();

        assert_ne!(chat_v1.fingerprint_crc64, chat_v2.fingerprint_crc64);
        assert_eq!(chat_v1.fingerprint_hex().len(), 16);
    }

    #[test]
    fn test_nested_participant_and_schema_evolution_v1_to_v2() {
        let registry = SchemaRegistry::default();
        let gm = Participant::human("u-gm-1", "sean_gm", "GM Sean", SessionRole::Gm);
        let player = Participant::human("u-pl-1", "sara_p", "Sara (Valkyrie)", SessionRole::Player);

        let metadata = registry
            .build_metadata(
                1,
                subjects::CHAT_MESSAGE,
                1,
                "harsh-reality-01",
                SessionMode::Ttrpg,
            )
            .unwrap();

        let msg_v1 = ChatMessageEvent {
            metadata,
            message_id: Uuid::now_v7().to_string(),
            sender: gm.clone(),
            visibility: MessageVisibility::Private,
            recipients: vec![player.clone()],
            channel_context: ChannelContext::GmNarration,
            reply_to_message_id: None,
            body: "You hear footsteps behind the bulkhead.".to_string(),
            char_count: 40,
            word_count: 6,
            composition_time_ms: 2150,
            is_edited: false,
        };

        let framed = registry
            .encode_framed(subjects::CHAT_MESSAGE, 1, &msg_v1)
            .unwrap();

        // 1. Decode using writer version (v1)
        let (hdr, roundtrip_v1): (_, ChatMessageEvent) =
            registry.decode_framed(&framed, None).unwrap();
        assert_eq!(hdr.schema_version, 1);
        assert_eq!(roundtrip_v1.sender.username, "sean_gm");
        assert_eq!(roundtrip_v1.recipients[0].display_name, "Sara (Valkyrie)");

        // 2. Evolve v1 message into v2 reader schema seamlessly
        let (hdr_v2, evolved_v2): (_, ChatMessageEventV2) =
            registry.decode_framed(&framed, Some(2)).unwrap();
        assert_eq!(hdr_v2.schema_version, 1); // Preserves original writer version!
        assert_eq!(evolved_v2.body, msg_v1.body);
        assert_eq!(evolved_v2.sentiment_score, None);
        assert_eq!(evolved_v2.detected_intent, None);
    }

    #[test]
    fn test_interaction_dynamics_and_ttrpg_mechanics_serialization() {
        let registry = SchemaRegistry::default();
        let speaker = Participant::human("u-1", "alex", "Alex", SessionRole::Host);
        let overlapped = Participant::human("u-2", "blair", "Blair", SessionRole::Participant);

        let dyn_meta = registry
            .build_metadata(
                10,
                subjects::INTERACTION_DYNAMICS,
                1,
                "meet-1",
                SessionMode::Meeting,
            )
            .unwrap();
        let dyn_event = InteractionDynamicsEvent {
            metadata: dyn_meta,
            speaker: speaker.clone(),
            turn_type: InteractionTurnType::InterruptionOverlap,
            duration_ms: 1840,
            rms_energy_mean: 0.42,
            rms_energy_peak: 0.88,
            pitch_hz_estimate: Some(165.5),
            overlapped_participants: vec![overlapped],
            preceding_silence_ms: 0,
            dominance_ratio: 0.61,
        };

        let framed_dyn = registry
            .encode_framed(subjects::INTERACTION_DYNAMICS, 1, &dyn_event)
            .unwrap();
        let (_, decoded_dyn): (_, InteractionDynamicsEvent) =
            registry.decode_framed(&framed_dyn, None).unwrap();
        assert_eq!(decoded_dyn.overlapped_participants.len(), 1);

        let mech_meta = registry
            .build_metadata(
                11,
                subjects::TTRPG_MECHANICS,
                1,
                "satya-1",
                SessionMode::Ttrpg,
            )
            .unwrap();
        let mech_event = TtrpgMechanicsEvent::evaluate_satya_task(
            mech_meta,
            speaker,
            vec![],
            "Lockpicking".to_string(),
            13,
            2,
            vec![4, 11, 17],
            Some(CombatStance::Maneuver),
            2,
            0.5,
        );
        let framed_mech = registry
            .encode_framed(subjects::TTRPG_MECHANICS, 1, &mech_event)
            .unwrap();
        let (_, decoded_mech): (_, TtrpgMechanicsEvent) =
            registry.decode_framed(&framed_mech, None).unwrap();
        assert_eq!(decoded_mech.successes_counted, 2);
        assert_eq!(decoded_mech.outcome_tier, OutcomeTier::Success);
    }

    #[test]
    fn test_disconnect_and_logout_lifecycle_and_webrtc_events() {
        let registry = SchemaRegistry::default();
        let player = Participant::human("u-pl-1", "sara_p", "Sara (Valkyrie)", SessionRole::Player);

        for lifecycle_type in [
            LifecycleEventType::ParticipantLeft,
            LifecycleEventType::ParticipantDisconnected,
            LifecycleEventType::UserLoggedOut,
        ] {
            let meta = registry
                .build_metadata(
                    20,
                    subjects::SESSION_LIFECYCLE,
                    1,
                    "harsh-reality-01",
                    SessionMode::Ttrpg,
                )
                .unwrap();
            let ev = SessionLifecycleEvent {
                metadata: meta,
                lifecycle_type,
                actor: player.clone(),
                target_participant: None,
                password_protected: true,
                roster_snapshot: vec![],
                reason: Some("User disconnected from session".to_string()),
            };
            let framed = registry
                .encode_framed(subjects::SESSION_LIFECYCLE, 1, &ev)
                .unwrap();
            let (_, decoded): (_, SessionLifecycleEvent) =
                registry.decode_framed(&framed, None).unwrap();
            assert_eq!(decoded.lifecycle_type, lifecycle_type);
        }

        let tel_meta = registry
            .build_metadata(
                21,
                subjects::WEBRTC_TELEMETRY,
                1,
                "harsh-reality-01",
                SessionMode::Ttrpg,
            )
            .unwrap();
        let tel_ev = WebRtcTelemetryEvent {
            metadata: tel_meta,
            local_participant: player,
            remote_participant: None,
            event_kind: WebRtcEventKind::PeerDisconnected,
            ice_candidate_type: None,
            rtt_ms: None,
            jitter_ms: None,
            packet_loss_rate: None,
            available_outgoing_bitrate_bps: None,
            audio_muted: true,
            video_muted: true,
        };
        let framed_tel = registry
            .encode_framed(subjects::WEBRTC_TELEMETRY, 1, &tel_ev)
            .unwrap();
        let (_, decoded_tel): (_, WebRtcTelemetryEvent) =
            registry.decode_framed(&framed_tel, None).unwrap();
        assert_eq!(decoded_tel.event_kind, WebRtcEventKind::PeerDisconnected);
    }
}
