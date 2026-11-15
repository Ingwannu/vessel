# Paseo 0.15.4 shared entities and cross-area connections

Companion to [CATALOGUE.md](CATALOGUE.md). All IDs refer to catalogue rows. Scope is the Paseo survey; these are Paseo relationships even where Vessel is absent. Reads/writes/triggers below index explicit operations in the feature heading/body; a feature can occupy multiple roles. Local preferences/drafts are distinguished from host-owned entities by the referenced feature. A trigger means observed events, subscriptions, hooks, callbacks or state-dependent refresh, not necessarily a write. Pure section indexes are excluded from entity roles.

## Shared entities

| Entity | Read by feature IDs | Written by feature IDs | Feature IDs triggered by changes |
|---|---|---|---|
| agent/session | F-A1, F-A3, F-A4, F-A5, F-A6, F-A9, F-A10, F-A11, F-A12, F-A13, F-A15, F-A17, F-A18, F-A19, F-A41, F-A63, F-A64, F-A66, F-A69, F-A70, F-A71, F-A73, F-A76, F-A78, F-A84, F-A86, F-A90, F-A91, F-A92, F-A93, F-A94, F-A95, F-A96, F-A97, F-A99, F-A101, F-A103, F-A105, F-A106, F-A113, F-A134, F-A136, F-A137, F-A138, F-A140, F-A141, F-A142, F-A143, F-A145, F-A153, F-A154, F-A155, F-A159, F-A160, F-A161, F-A165, F-A166, F-A167, F-A169, F-A176, F-A177, F-A179, F-A191, F-A212, F-A216, F-A220, F-B2, F-B6, F-B16, F-B18, F-B25, F-B26, F-B43, F-B44, F-B47, F-B49, F-B50, F-B56, F-B58, F-B59, F-B66, F-B68, F-B72, F-B75, F-B85, F-B90, F-B91, F-B92, F-C3, F-C11, F-C14, F-C21, F-C30, F-C33, F-C35, F-C39, F-C42, F-C50, F-C56, F-C57, F-C60, F-C69, F-C75, F-C77, F-C80, F-C84, F-C99, F-C188, F-C190, F-C206, F-D4, F-D6, F-D8, F-D12, F-E16, F-E22, F-E29, F-E32, F-E36, F-E39, F-E40, F-E43, F-E48, F-E49, F-E52, F-F2, F-F16, F-F17, F-G19, F-G20, F-G23, F-G27, F-G30, F-G34, F-G36, F-G37, F-G38, F-H8, F-H11, F-H16, F-H30, F-H33, F-H41, F-H57, F-H58, F-H60, F-H73, F-I1, F-I2, F-I3, F-I4, F-I6, F-I7, F-I8, F-I11, F-I13, F-I16, F-I18, F-I19, F-I32, F-I40, F-I46, F-J3, F-J6, F-J17, F-J26, F-J29, F-J37, F-J38, F-J43, F-J51, F-J59, F-J65, F-J81, F-J82, F-J97, F-K2, F-K3, F-K14, F-K17, F-K30, F-K33, F-K34, F-K35, F-K42, F-K43, F-K46, F-L2, F-L19, F-L26, F-L40, F-M8, F-M17, F-A260, F-A261, F-A263, F-A264, F-A265, F-A266, F-A267, F-A268, F-A269, F-C234, F-A270, F-N2, F-N3 | F-A1, F-A3, F-A4, F-A5, F-A6, F-A7, F-A8, F-A9, F-A10, F-A11, F-A13, F-A14, F-A15, F-A17, F-A19, F-A22, F-A28, F-A29, F-A30, F-A31, F-A32, F-A38, F-A41, F-A69, F-A73, F-A76, F-A82, F-A89, F-A90, F-A91, F-A92, F-A93, F-A94, F-A95, F-A96, F-A97, F-A98, F-A99, F-A101, F-A108, F-A113, F-A138, F-A140, F-A150, F-A153, F-A154, F-A161, F-A166, F-A167, F-A173, F-A177, F-A179, F-B2, F-B3, F-B4, F-B10, F-B11, F-B12, F-B15, F-B16, F-B17, F-B18, F-B20, F-B22, F-B25, F-B26, F-B27, F-B43, F-B44, F-B47, F-B49, F-B50, F-B52, F-B56, F-B58, F-B62, F-B65, F-B66, F-B68, F-B72, F-B74, F-B85, F-B89, F-B90, F-B91, F-B93, F-B99, F-C6, F-C17, F-C18, F-C21, F-C23, F-C33, F-C35, F-C36, F-C38, F-C39, F-C41, F-C42, F-C45, F-C48, F-C51, F-C53, F-C56, F-C60, F-C65, F-C66, F-C68, F-C69, F-C74, F-C75, F-C77, F-C80, F-C81, F-C84, F-C90, F-C95, F-C99, F-C147, F-C190, F-C208, F-C209, F-C222, F-C229, F-C230, F-D3, F-D6, F-D12, F-E22, F-E29, F-E32, F-E36, F-E39, F-E40, F-E46, F-E49, F-E52, F-F5, F-F14, F-F16, F-F32, F-G6, F-G14, F-G15, F-G19, F-G20, F-G21, F-G27, F-G30, F-G35, F-G36, F-H6, F-H21, F-H33, F-H39, F-H41, F-H52, F-H55, F-I2, F-I3, F-I6, F-I7, F-I8, F-I9, F-I10, F-I11, F-I13, F-I19, F-I26, F-I32, F-I38, F-I47, F-J3, F-J6, F-J9, F-J14, F-J26, F-J37, F-J38, F-J44, F-J59, F-J65, F-J69, F-J82, F-K13, F-K14, F-K22, F-K24, F-K33, F-K34, F-K41, F-K43, F-L2, F-L5, F-L19, F-L22, F-L23, F-L24, F-L28, F-L40, F-M7, F-M13, F-A260, F-A262, F-A263, F-J102, F-A265, F-A266, F-A267, F-A268, F-A269, F-C234, F-L79, F-A270, F-N2, F-N3 | F-A1, F-A2, F-A3, F-A4, F-A5, F-A6, F-A8, F-A9, F-A10, F-A11, F-A14, F-A15, F-A16, F-A17, F-A18, F-A19, F-A63, F-A69, F-A71, F-A78, F-A87, F-A94, F-A99, F-A101, F-A103, F-A105, F-A106, F-A108, F-A113, F-A134, F-A135, F-A137, F-A138, F-A140, F-A143, F-A150, F-A152, F-A153, F-A154, F-A157, F-A159, F-A160, F-A161, F-A165, F-A166, F-A167, F-A169, F-A177, F-A181, F-A191, F-B3, F-B4, F-B9, F-B10, F-B11, F-B12, F-B15, F-B16, F-B17, F-B18, F-B20, F-B22, F-B43, F-B47, F-B49, F-B50, F-B51, F-B52, F-B56, F-B58, F-B59, F-B62, F-B63, F-B64, F-B65, F-B68, F-B72, F-B85, F-B89, F-B90, F-B91, F-B92, F-B93, F-B95, F-C33, F-C35, F-C36, F-C39, F-C42, F-C45, F-C48, F-C51, F-C57, F-C60, F-C65, F-C69, F-C74, F-C98, F-C99, F-C206, F-D4, F-D6, F-D8, F-D10, F-D12, F-E16, F-E22, F-E32, F-E39, F-E40, F-E46, F-E49, F-F2, F-F4, F-F11, F-F16, F-F17, F-F18, F-G6, F-G19, F-G21, F-G23, F-G30, F-G34, F-G36, F-G37, F-G47, F-H3, F-H6, F-H16, F-H18, F-H21, F-H33, F-H41, F-H55, F-H57, F-H58, F-I1, F-I2, F-I3, F-I6, F-I7, F-I9, F-I11, F-I13, F-I16, F-I18, F-I19, F-I26, F-I40, F-J2, F-J3, F-J6, F-J8, F-J9, F-J13, F-J14, F-J25, F-J26, F-J29, F-J31, F-J32, F-J37, F-J38, F-J44, F-J51, F-J56, F-J59, F-J65, F-J69, F-J81, F-K2, F-K3, F-K8, F-K14, F-K30, F-K33, F-K36, F-K41, F-K42, F-K43, F-L2, F-L19, F-L22, F-L28, F-L61, F-M8, F-M13, F-M14, F-M15, F-M17, F-A260, F-A261, F-A262, F-A263, F-J102, F-A265, F-A266, F-A267, F-A268, F-A269, F-C234, F-A270, F-N2, F-N3 |
| workspace/project | F-A1, F-A3, F-A4, F-A5, F-A6, F-A10, F-A11, F-A17, F-A18, F-A19, F-A40, F-A82, F-A91, F-A94, F-A99, F-A113, F-A160, F-A198, F-A220, F-A250, F-B25, F-B37, F-B47, F-B56, F-B71, F-B83, F-B86, F-B87, F-C1, F-C2, F-C3, F-C5, F-C8, F-C11, F-C14, F-C15, F-C17, F-C18, F-C20, F-C21, F-C23, F-C24, F-C26, F-C27, F-C29, F-C30, F-C32, F-C33, F-C35, F-C38, F-C39, F-C41, F-C42, F-C44, F-C45, F-C47, F-C48, F-C50, F-C51, F-C53, F-C54, F-C56, F-C57, F-C60, F-C63, F-C64, F-C65, F-C66, F-C69, F-C71, F-C72, F-C73, F-C74, F-C75, F-C76, F-C77, F-C78, F-C80, F-C81, F-C84, F-C86, F-C87, F-C89, F-C90, F-C92, F-C93, F-C95, F-C96, F-C97, F-C98, F-C99, F-C102, F-C103, F-C104, F-C105, F-C106, F-C108, F-C113, F-C124, F-C128, F-C135, F-C137, F-C143, F-C145, F-C169, F-C173, F-C174, F-C180, F-C181, F-C183, F-C187, F-C188, F-C189, F-C190, F-C196, F-C199, F-C200, F-C201, F-C202, F-C203, F-C204, F-C205, F-C206, F-C212, F-C213, F-C214, F-C221, F-C231, F-C232, F-C233, F-D4, F-D5, F-D6, F-D7, F-D8, F-D9, F-D10, F-D13, F-D15, F-D17, F-D19, F-D22, F-D23, F-D54, F-E6, F-E10, F-E11, F-E13, F-E15, F-E16, F-E19, F-E21, F-E22, F-E26, F-E27, F-E29, F-E32, F-E33, F-E34, F-E35, F-E36, F-E38, F-E39, F-E40, F-E41, F-E42, F-E43, F-E44, F-E48, F-E52, F-E77, F-E78, F-E80, F-E83, F-F4, F-F9, F-F10, F-F12, F-F14, F-F16, F-F17, F-G4, F-G6, F-G8, F-G9, F-G19, F-G20, F-G23, F-G26, F-G27, F-G29, F-G30, F-G31, F-G34, F-H11, F-H16, F-H30, F-H33, F-H41, F-H55, F-H58, F-H88, F-I1, F-I7, F-I11, F-I13, F-I14, F-I19, F-I32, F-J3, F-J11, F-J26, F-J29, F-J37, F-J38, F-J43, F-J76, F-J92, F-K2, F-K3, F-K4, F-K8, F-K9, F-K10, F-K12, F-K13, F-K14, F-K15, F-K16, F-K17, F-K19, F-K22, F-K23, F-K30, F-K32, F-K33, F-K34, F-K35, F-K41, F-K42, F-K43, F-K46, F-K53, F-L1, F-L2, F-L40, F-A269, F-C234, F-L72, F-L76, F-L77, F-C235, F-N1, F-N3 | F-A1, F-A3, F-A4, F-A5, F-A6, F-A7, F-A9, F-A10, F-A11, F-A13, F-A14, F-A15, F-A19, F-A29, F-A36, F-A37, F-A82, F-A91, F-A94, F-A99, F-A100, F-A113, F-A160, F-A161, F-A198, F-A250, F-B9, F-B22, F-B25, F-B27, F-B43, F-B47, F-B53, F-B56, F-B71, F-B72, F-B74, F-B76, F-B83, F-C2, F-C4, F-C5, F-C6, F-C8, F-C9, F-C17, F-C18, F-C19, F-C20, F-C21, F-C23, F-C24, F-C26, F-C27, F-C29, F-C30, F-C33, F-C34, F-C35, F-C36, F-C37, F-C38, F-C39, F-C41, F-C42, F-C43, F-C44, F-C45, F-C47, F-C48, F-C49, F-C50, F-C51, F-C52, F-C53, F-C54, F-C56, F-C57, F-C58, F-C59, F-C60, F-C62, F-C63, F-C65, F-C66, F-C67, F-C68, F-C69, F-C71, F-C72, F-C73, F-C74, F-C75, F-C77, F-C78, F-C80, F-C81, F-C83, F-C84, F-C86, F-C87, F-C90, F-C93, F-C95, F-C96, F-C98, F-C99, F-C104, F-C107, F-C108, F-C109, F-C110, F-C115, F-C116, F-C121, F-C122, F-C125, F-C126, F-C127, F-C129, F-C132, F-C137, F-C144, F-C145, F-C146, F-C147, F-C148, F-C149, F-C150, F-C151, F-C152, F-C153, F-C154, F-C161, F-C162, F-C163, F-C164, F-C165, F-C169, F-C170, F-C171, F-C172, F-C173, F-C174, F-C175, F-C180, F-C181, F-C182, F-C183, F-C189, F-C190, F-C191, F-C192, F-C195, F-C196, F-C199, F-C200, F-C201, F-C202, F-C203, F-C212, F-C213, F-C214, F-C215, F-C222, F-C229, F-C230, F-D3, F-D6, F-D7, F-D8, F-D9, F-D10, F-D15, F-D17, F-D18, F-D19, F-D20, F-D22, F-D23, F-D72, F-E4, F-E10, F-E11, F-E13, F-E19, F-E21, F-E22, F-E29, F-E32, F-E35, F-E36, F-E37, F-E38, F-E39, F-E40, F-E43, F-E44, F-E46, F-E52, F-E53, F-E78, F-E79, F-E81, F-F1, F-F4, F-F5, F-F10, F-F14, F-F16, F-F18, F-G3, F-G4, F-G6, F-G8, F-G9, F-G14, F-G15, F-G19, F-G20, F-G26, F-G27, F-G28, F-G29, F-G30, F-G31, F-G34, F-G35, F-G36, F-G37, F-H16, F-H33, F-H41, F-I2, F-I7, F-I13, F-I14, F-I26, F-I32, F-J26, F-J32, F-J37, F-J38, F-J92, F-K13, F-K14, F-K19, F-K22, F-K23, F-K24, F-K26, F-K33, F-K34, F-K35, F-K39, F-K41, F-K42, F-K43, F-K53, F-L2, F-L27, F-L40, F-L60, F-M13, F-M21, F-A269, F-C234, F-L72, F-L76, F-L77, F-C235, F-N3 | F-A1, F-A3, F-A4, F-A5, F-A6, F-A10, F-A11, F-A14, F-A15, F-A17, F-A18, F-A19, F-A91, F-A94, F-A99, F-A113, F-A136, F-A137, F-B43, F-B47, F-B53, F-B56, F-B58, F-B71, F-B75, F-B83, F-C2, F-C5, F-C14, F-C18, F-C29, F-C33, F-C35, F-C36, F-C39, F-C42, F-C45, F-C48, F-C51, F-C57, F-C59, F-C60, F-C62, F-C63, F-C65, F-C69, F-C71, F-C72, F-C74, F-C75, F-C76, F-C77, F-C78, F-C81, F-C86, F-C92, F-C93, F-C98, F-C99, F-C149, F-C154, F-C184, F-C204, F-C205, F-C206, F-D2, F-D4, F-D6, F-D7, F-D8, F-D9, F-D13, F-D15, F-D17, F-D18, F-D19, F-D20, F-D23, F-D55, F-E13, F-E15, F-E16, F-E19, F-E21, F-E26, F-E27, F-E32, F-E35, F-E39, F-E40, F-E42, F-E46, F-E78, F-E79, F-F4, F-F9, F-F12, F-F14, F-F15, F-F16, F-F17, F-F18, F-G3, F-G4, F-G6, F-G8, F-G9, F-G17, F-G19, F-G20, F-G26, F-G27, F-G28, F-G29, F-G30, F-G31, F-G34, F-G35, F-G36, F-G37, F-H30, F-H33, F-H58, F-I1, F-I2, F-I7, F-I14, F-I26, F-J3, F-J14, F-J26, F-J27, F-J29, F-J37, F-J38, F-J47, F-K2, F-K3, F-K4, F-K14, F-K19, F-K23, F-K26, F-K30, F-K33, F-K34, F-K36, F-K41, F-K42, F-K43, F-L2, F-L40, F-M13, F-M17, F-J102, F-A269, F-L77, F-C235, F-N3, F-N4 |
| host/node | F-A4, F-A5, F-A6, F-A10, F-A17, F-A18, F-A19, F-A91, F-A93, F-A99, F-A113, F-A140, F-A166, F-A176, F-B25, F-B56, F-B58, F-B66, F-B68, F-B71, F-B75, F-B90, F-C1, F-C2, F-C3, F-C5, F-C11, F-C12, F-C18, F-C27, F-C29, F-C30, F-C42, F-C45, F-C47, F-C54, F-C56, F-C57, F-C60, F-C69, F-C71, F-C72, F-C90, F-C97, F-C98, F-C99, F-C102, F-C103, F-C104, F-C105, F-C106, F-C128, F-C135, F-C137, F-C174, F-C176, F-C179, F-C231, F-C232, F-C233, F-D6, F-D10, F-D12, F-D13, F-D14, F-D15, F-D17, F-E4, F-E5, F-E10, F-E12, F-E18, F-E22, F-E28, F-E34, F-E42, F-E43, F-E49, F-E54, F-F2, F-F4, F-F6, F-F9, F-F11, F-F12, F-G2, F-G4, F-G6, F-G8, F-G9, F-G10, F-G12, F-G19, F-G26, F-G27, F-G31, F-G36, F-G37, F-G38, F-H1, F-H2, F-H3, F-H4, F-H5, F-H6, F-H7, F-H8, F-H10, F-H11, F-H15, F-H16, F-H21, F-H25, F-H26, F-H28, F-H30, F-H31, F-H33, F-H34, F-H35, F-H37, F-H38, F-H41, F-H42, F-H46, F-H52, F-H55, F-H57, F-H58, F-H59, F-H61, F-H62, F-H65, F-H70, F-H73, F-H86, F-H88, F-H91, F-H117, F-H118, F-H124, F-I1, F-I4, F-I5, F-I7, F-I8, F-I11, F-I17, F-I19, F-I30, F-I32, F-I40, F-J3, F-J26, F-J29, F-J32, F-J51, F-J59, F-J63, F-J65, F-J72, F-J74, F-J76, F-J81, F-J82, F-J99, F-J101, F-K2, F-K4, F-K9, F-K14, F-K17, F-K19, F-K22, F-K23, F-K30, F-K33, F-K42, F-K43, F-K44, F-K46, F-L1, F-L2, F-L18, F-L19, F-L30, F-L32, F-L35, F-L37, F-L38, F-L40, F-L58, F-M6, F-M8, F-M9, F-M12, F-M16, F-M17, F-M18, F-M19, F-A260, F-A264, F-A265, F-A266, F-A267, F-A268, F-A269, F-C234, F-L73, F-L76, F-L79, F-L80, F-C235, F-A270, F-N2 | F-A4, F-A5, F-A6, F-A15, F-A19, F-A106, F-A167, F-B10, F-B11, F-B25, F-B43, F-B52, F-B53, F-B56, F-B66, F-B68, F-B71, F-B72, F-B90, F-B91, F-B92, F-B93, F-C2, F-C5, F-C20, F-C21, F-C27, F-C29, F-C42, F-C47, F-C48, F-C53, F-C54, F-C56, F-C57, F-C60, F-C69, F-C71, F-C72, F-C81, F-C83, F-C86, F-C90, F-C93, F-C96, F-C98, F-C99, F-C104, F-C127, F-C137, F-C166, F-C174, F-C176, F-C179, F-C215, F-D6, F-D10, F-D12, F-D17, F-D20, F-E3, F-E10, F-E22, F-E46, F-E49, F-E54, F-F7, F-F10, F-F11, F-F14, F-G2, F-G3, F-G4, F-G6, F-G8, F-G9, F-G10, F-G12, F-G19, F-G26, F-G27, F-G29, F-G31, F-G36, F-G37, F-G48, F-H2, F-H3, F-H4, F-H6, F-H7, F-H10, F-H11, F-H15, F-H16, F-H20, F-H21, F-H25, F-H26, F-H28, F-H30, F-H31, F-H33, F-H35, F-H37, F-H38, F-H39, F-H41, F-H42, F-H44, F-H46, F-H47, F-H48, F-H50, F-H51, F-H52, F-H53, F-H55, F-H59, F-H61, F-H66, F-H70, F-H80, F-H89, F-H90, F-H97, F-H108, F-H109, F-H111, F-I5, F-I7, F-I10, F-I11, F-I19, F-I25, F-I32, F-J26, F-J32, F-J38, F-J59, F-J60, F-J63, F-J65, F-J76, F-J82, F-K14, F-K19, F-K22, F-K23, F-K26, F-K33, F-K41, F-K43, F-L2, F-L17, F-L19, F-L24, F-L28, F-L30, F-L35, F-L38, F-L58, F-L69, F-M6, F-M8, F-M9, F-M12, F-M17, F-M18, F-M19, F-M20, F-A260, F-A265, F-A266, F-A267, F-A268, F-C234, F-L72, F-L73, F-L76, F-L78, F-L79, F-L80, F-C235, F-A270, F-N2, F-N3, F-N4 | F-A1, F-A3, F-A4, F-A6, F-A9, F-A10, F-A15, F-A17, F-A18, F-A19, F-A135, F-A137, F-A143, F-A154, F-A166, F-A167, F-A169, F-B9, F-B10, F-B11, F-B16, F-B17, F-B20, F-B22, F-B25, F-B50, F-B52, F-B53, F-B56, F-B58, F-B62, F-B66, F-B68, F-B72, F-B77, F-B86, F-B90, F-B91, F-B93, F-B95, F-C2, F-C18, F-C53, F-C57, F-C63, F-C71, F-C93, F-C98, F-C99, F-D6, F-D12, F-D13, F-D15, F-D17, F-D20, F-D57, F-E2, F-E4, F-E10, F-E18, F-E22, F-E28, F-E34, F-E42, F-E49, F-E54, F-F2, F-F4, F-F6, F-F7, F-F9, F-F10, F-F11, F-F12, F-F14, F-F18, F-G4, F-G6, F-G8, F-G9, F-G10, F-G12, F-G19, F-G26, F-G28, F-G29, F-G31, F-G36, F-G37, F-G48, F-H3, F-H6, F-H7, F-H8, F-H10, F-H11, F-H15, F-H16, F-H18, F-H20, F-H21, F-H25, F-H26, F-H28, F-H30, F-H31, F-H33, F-H34, F-H35, F-H38, F-H41, F-H42, F-H46, F-H47, F-H48, F-H51, F-H52, F-H55, F-H57, F-H58, F-H59, F-H61, F-H70, F-H79, F-I3, F-I5, F-I6, F-I7, F-I8, F-I11, F-I17, F-I19, F-I25, F-I28, F-I30, F-I40, F-J3, F-J8, F-J9, F-J21, F-J25, F-J26, F-J29, F-J31, F-J32, F-J38, F-J51, F-J59, F-J65, F-J72, F-J76, F-J77, F-J81, F-K2, F-K4, F-K14, F-K19, F-K23, F-K26, F-K30, F-K33, F-K41, F-K42, F-K43, F-K44, F-L2, F-L17, F-L19, F-L28, F-L35, F-L38, F-L39, F-M9, F-M17, F-M18, F-M19, F-A260, F-A263, F-A265, F-A268, F-C234, F-L72, F-L76, F-L78, F-L79, F-C235, F-N1, F-N2, F-N3, F-K63 |
| timeline item | F-A1, F-A3, F-A5, F-A9, F-A10, F-A11, F-A12, F-A13, F-A73, F-A101, F-A113, F-A133, F-A134, F-A135, F-A136, F-A137, F-A138, F-A140, F-A141, F-A142, F-A143, F-A145, F-A146, F-A147, F-A153, F-A154, F-A155, F-A159, F-A160, F-A161, F-A162, F-A163, F-A164, F-A165, F-A166, F-A167, F-A169, F-A176, F-A179, F-A180, F-A182, F-A183, F-A205, F-A212, F-A216, F-A219, F-A222, F-A226, F-A249, F-B14, F-B75, F-B86, F-D3, F-D12, F-E7, F-E10, F-E12, F-E16, F-E22, F-E26, F-E28, F-E32, F-E49, F-E61, F-E63, F-H33, F-I2, F-J3, F-J6, F-J51, F-M9, F-M17, F-A260, F-A263, F-L79, F-L80, F-A270, F-N3 | F-A3, F-A5, F-A8, F-A9, F-A10, F-A11, F-A12, F-A99, F-A113, F-A138, F-A140, F-A141, F-A150, F-A153, F-A154, F-A160, F-A161, F-A165, F-A166, F-A167, F-A171, F-A176, F-A177, F-A179, F-A193, F-A211, F-A226, F-B15, F-B16, F-B50, F-D12, F-E10, F-E12, F-E32, F-E49, F-E61, F-H33, F-I2, F-M9, F-M13, F-A262, F-L79, F-L80, F-A270, F-N3 | F-A1, F-A3, F-A8, F-A9, F-A10, F-A11, F-A87, F-A113, F-A134, F-A137, F-A138, F-A140, F-A143, F-A148, F-A150, F-A152, F-A153, F-A154, F-A155, F-A157, F-A159, F-A162, F-A163, F-A164, F-A165, F-A166, F-A167, F-A169, F-A170, F-A171, F-A177, F-A180, F-A183, F-A199, F-A226, F-B3, F-B15, F-B16, F-B18, F-B22, F-B50, F-B52, F-D12, F-E7, F-E12, F-E16, F-E18, F-E27, F-E28, F-E32, F-E49, F-E61, F-H31, F-H33, F-J3, F-M9, F-M13, F-M14, F-A262, F-A263, F-L79, F-A270, F-N3 |
| permission/question | F-A1, F-A5, F-A17, F-A73, F-A76, F-A78, F-A82, F-A85, F-A100, F-A103, F-A109, F-A113, F-A117, F-A131, F-A160, F-A161, F-A167, F-B47, F-B50, F-B66, F-B92, F-C189, F-D10, F-D12, F-E5, F-E11, F-E62, F-G4, F-H11, F-H16, F-H25, F-H49, F-H58, F-H73, F-J3, F-J6, F-J17, F-J19, F-J23, F-J38, F-K14, F-K30, F-L14, F-A260, F-A261, F-A262, F-A263, F-A264, F-A265, F-N2, F-N3 | F-A3, F-A5, F-A9, F-A17, F-A77, F-A82, F-A85, F-A100, F-A109, F-A113, F-A161, F-B9, F-B10, F-B18, F-B22, F-B47, F-B66, F-B91, F-B102, F-C66, F-C189, F-D10, F-D12, F-E11, F-E22, F-E27, F-E62, F-G4, F-G6, F-G27, F-H11, F-H16, F-H21, F-H39, F-H49, F-I2, F-J3, F-J14, F-J19, F-J20, F-J38, F-J44, F-K35, F-M13, F-A260, F-A261, F-A262, F-A263, F-J102, F-L79, F-N2, F-N6 | F-A1, F-A2, F-A3, F-A9, F-A17, F-A76, F-A78, F-A82, F-A99, F-A103, F-A112, F-A113, F-A135, F-A137, F-A152, F-A160, F-A163, F-A169, F-A192, F-B10, F-B18, F-B47, F-B49, F-B50, F-B64, F-B91, F-B92, F-B95, F-D12, F-E27, F-F4, F-G4, F-G6, F-H16, F-H18, F-H21, F-H25, F-H58, F-I13, F-J2, F-J3, F-J14, F-J19, F-J20, F-J31, F-J38, F-J44, F-J47, F-J89, F-K14, F-M13, F-A260, F-A261, F-A262, F-A263, F-J102, F-A264, F-N2, F-N3 |
| attachment | F-A3, F-A4, F-A11, F-A101, F-A113, F-B2, F-B6, F-B20, F-B24, F-B25, F-B37, F-B43, F-B44, F-B47, F-B50, F-B56, F-B58, F-B59, F-B70, F-B71, F-B72, F-B75, F-B79, F-B81, F-B82, F-B83, F-C30, F-C33, F-C96, F-D13, F-E6, F-E27, F-E29, F-E32, F-E35, F-E36, F-E40, F-E48, F-E49, F-E52, F-E77, F-E83, F-F6, F-M17 | F-A3, F-A4, F-A11, F-A113, F-B2, F-B3, F-B7, F-B20, F-B24, F-B25, F-B43, F-B47, F-B50, F-B52, F-B56, F-B58, F-B71, F-B72, F-B73, F-B74, F-B76, F-B77, F-B79, F-B81, F-B82, F-B83, F-B95, F-B112, F-B115, F-C45, F-C95, F-C96, F-E29, F-E32, F-E35, F-E36, F-E39, F-E40, F-E48, F-E49, F-E52, F-F7, F-F11, F-G9, F-G35, F-H41, F-H44 | F-A3, F-A4, F-A11, F-A113, F-B2, F-B3, F-B9, F-B10, F-B20, F-B25, F-B44, F-B47, F-B50, F-B52, F-B56, F-B58, F-B59, F-B71, F-B72, F-B73, F-B75, F-B77, F-B82, F-B83, F-B87, F-C33, F-D13, F-E27, F-E32, F-E35, F-E40, F-E48, F-E49, F-F6 |
| attention/notification | F-A1, F-A3, F-A9, F-A17, F-A19, F-A101, F-A105, F-A113, F-A134, F-A166, F-A169, F-B6, F-B71, F-B86, F-C24, F-C30, F-C33, F-C64, F-C65, F-C66, F-C72, F-C74, F-C81, F-C187, F-C188, F-C189, F-C190, F-D12, F-D15, F-D17, F-D22, F-E10, F-E27, F-E49, F-G6, F-G31, F-H16, F-H57, F-I11, F-J3, F-J12, F-J13, F-J14, F-J15, F-J16, F-J17, F-J19, F-J20, F-J23, F-J26, F-J29, F-J32, F-J37, F-J38, F-J50, F-J51, F-J92, F-K14, F-K17, F-K23, F-K30, F-K53, F-L2, F-L14, F-L18, F-A266, F-J103 | F-A1, F-A3, F-A6, F-A9, F-A17, F-A19, F-A60, F-A113, F-A153, F-A171, F-B3, F-B15, F-B71, F-B93, F-C24, F-C42, F-C65, F-C66, F-C72, F-C74, F-C189, F-C190, F-D12, F-D17, F-D19, F-D70, F-E27, F-E46, F-E49, F-E84, F-F5, F-G6, F-G8, F-G9, F-G31, F-G34, F-G48, F-H41, F-H44, F-H49, F-H122, F-I2, F-I6, F-I11, F-I19, F-J3, F-J4, F-J6, F-J11, F-J14, F-J19, F-J20, F-J26, F-J32, F-J37, F-J38, F-J39, F-J44, F-J91, F-J92, F-K14, F-K22, F-K23, F-K24, F-K53, F-L2, F-L14, F-J102, F-A266, F-A270 | F-A1, F-A2, F-A3, F-A6, F-A9, F-A16, F-A17, F-A19, F-A61, F-A103, F-A113, F-A137, F-A152, F-A153, F-A157, F-A163, F-A166, F-A169, F-A171, F-A192, F-B3, F-B15, F-B50, F-B62, F-B65, F-B86, F-B93, F-C42, F-C72, F-D10, F-D12, F-D17, F-D19, F-D22, F-E22, F-E27, F-E46, F-E49, F-F18, F-G3, F-G4, F-G6, F-G8, F-G31, F-G48, F-H18, F-H25, F-H42, F-H47, F-H62, F-I2, F-I11, F-I13, F-I19, F-J2, F-J3, F-J4, F-J6, F-J8, F-J9, F-J13, F-J14, F-J16, F-J19, F-J20, F-J21, F-J25, F-J26, F-J29, F-J30, F-J31, F-J32, F-J33, F-J34, F-J35, F-J37, F-J38, F-J39, F-J44, F-J47, F-J51, F-J56, F-J88, F-K14, F-K23, F-K43, F-L2, F-A263, F-J102, F-J103 |
| settings | F-A1, F-A3, F-A4, F-A9, F-A69, F-A76, F-A84, F-A85, F-A86, F-A89, F-A94, F-A99, F-A100, F-A101, F-A103, F-A105, F-A108, F-A113, F-A124, F-B47, F-B58, F-B59, F-B66, F-B68, F-B86, F-B91, F-B109, F-C23, F-C24, F-C26, F-C27, F-C30, F-C33, F-C41, F-C45, F-C47, F-C87, F-C98, F-C128, F-C203, F-C231, F-C232, F-D6, F-D15, F-E4, F-E10, F-E32, F-E49, F-E52, F-E54, F-F12, F-F33, F-G9, F-G10, F-G12, F-G26, F-G27, F-G30, F-G31, F-G36, F-H2, F-H3, F-H10, F-H11, F-H15, F-H16, F-H28, F-H30, F-H37, F-H42, F-H46, F-H47, F-H49, F-H55, F-H58, F-H69, F-H70, F-H88, F-H124, F-I1, F-I2, F-I3, F-I7, F-I11, F-I13, F-I17, F-I18, F-I19, F-I27, F-I42, F-J14, F-J17, F-J20, F-J23, F-J32, F-J59, F-J63, F-J65, F-J72, F-J74, F-J76, F-J81, F-K2, F-K9, F-K13, F-K15, F-K18, F-K19, F-K20, F-K30, F-K33, F-K34, F-K36, F-K37, F-L1, F-L2, F-L4, F-L11, F-L16, F-L18, F-L19, F-L26, F-L28, F-L30, F-L31, F-L32, F-L40, F-L58, F-M8, F-M12, F-M18, F-M19, F-A263, F-A265, F-A266, F-A267, F-A268, F-C234, F-L72, F-L73, F-L75, F-L77, F-L78 | F-A3, F-A4, F-A7, F-A9, F-A10, F-A31, F-A38, F-A69, F-A73, F-A85, F-A89, F-A94, F-A98, F-A99, F-A100, F-A106, F-A112, F-A113, F-A130, F-B4, F-B27, F-B32, F-B44, F-B45, F-B47, F-B50, F-B64, F-B66, F-B68, F-B91, F-B93, F-B100, F-B109, F-C17, F-C23, F-C24, F-C26, F-C27, F-C39, F-C42, F-C47, F-C48, F-C72, F-C75, F-C129, F-C132, F-C154, F-C195, F-C203, F-D6, F-D15, F-D19, F-E10, F-E21, F-E32, F-E47, F-E49, F-E52, F-E54, F-F13, F-G6, F-G8, F-G9, F-G10, F-G12, F-G26, F-G27, F-G28, F-G29, F-G31, F-G34, F-G36, F-H2, F-H3, F-H6, F-H10, F-H11, F-H20, F-H21, F-H25, F-H26, F-H28, F-H30, F-H42, F-H44, F-H46, F-H47, F-H48, F-H49, F-H51, F-H54, F-H55, F-H66, F-H70, F-H89, F-H109, F-I2, F-I3, F-I7, F-I10, F-I11, F-I19, F-I27, F-I38, F-I42, F-J20, F-J21, F-J32, F-J59, F-J63, F-J76, F-K22, F-K26, F-K27, F-K34, F-K35, F-K39, F-L2, F-L19, F-L28, F-L30, F-L40, F-L58, F-M8, F-M12, F-M13, F-M19, F-A265, F-A266, F-A267, F-A268, F-C234, F-L72, F-L73, F-L74, F-L76, F-L78 | F-A1, F-A3, F-A4, F-A9, F-A10, F-A69, F-A82, F-A94, F-A98, F-A99, F-A100, F-A101, F-A103, F-A104, F-A105, F-A113, F-A167, F-B4, F-B45, F-B47, F-B50, F-B58, F-B64, F-B66, F-B68, F-B90, F-B91, F-B93, F-C33, F-C39, F-C42, F-C75, F-C154, F-D6, F-D8, F-D15, F-D22, F-E10, F-E26, F-E32, F-E49, F-E54, F-F9, F-F12, F-F13, F-F14, F-F18, F-G6, F-G8, F-G10, F-G26, F-G27, F-G28, F-G29, F-G34, F-G36, F-H6, F-H10, F-H11, F-H15, F-H16, F-H18, F-H20, F-H21, F-H25, F-H26, F-H28, F-H30, F-H42, F-H46, F-H47, F-H48, F-H51, F-H52, F-H55, F-H58, F-H70, F-I1, F-I2, F-I3, F-I7, F-I11, F-I13, F-I17, F-I19, F-J14, F-J20, F-J21, F-J32, F-J33, F-J59, F-J65, F-J76, F-J77, F-J81, F-K2, F-K9, F-K15, F-K26, F-K30, F-K33, F-K34, F-K35, F-K36, F-L2, F-L19, F-L28, F-L39, F-L40, F-M1, F-M8, F-M9, F-M12, F-M13, F-M15, F-M18, F-M19, F-A263, F-A265, F-A266, F-A267, F-C234, F-L74, F-L76, F-L77, F-L78, F-L80, F-N3 |
| provider/model/profile | F-A1, F-A3, F-A4, F-A5, F-A9, F-A10, F-A11, F-A12, F-A19, F-A49, F-A67, F-A68, F-A69, F-A70, F-A71, F-A72, F-A73, F-A74, F-A75, F-A76, F-A77, F-A78, F-A80, F-A82, F-A84, F-A85, F-A86, F-A90, F-A91, F-A92, F-A93, F-A94, F-A95, F-A96, F-A97, F-A99, F-A100, F-A101, F-A103, F-A105, F-A106, F-A108, F-A109, F-A110, F-A113, F-A118, F-A126, F-A127, F-A131, F-A133, F-A134, F-A135, F-A136, F-A137, F-A138, F-A140, F-A154, F-A160, F-A161, F-A162, F-A163, F-A164, F-A165, F-A167, F-A169, F-A171, F-A172, F-A176, F-A177, F-A179, F-A219, F-A220, F-A222, F-A224, F-A237, F-B6, F-B7, F-B16, F-B17, F-B24, F-B40, F-B43, F-B44, F-B50, F-B56, F-B57, F-B58, F-B66, F-B81, F-B109, F-C2, F-C3, F-C14, F-C15, F-C17, F-C30, F-C32, F-C33, F-C45, F-C51, F-C53, F-C54, F-C56, F-C57, F-C60, F-C63, F-C66, F-C71, F-C72, F-C77, F-C95, F-C96, F-C99, F-C143, F-C204, F-C233, F-D6, F-D12, F-D13, F-D15, F-D19, F-D22, F-D73, F-E5, F-E16, F-E30, F-F12, F-F13, F-G10, F-G12, F-G19, F-G20, F-G23, F-G61, F-G65, F-H1, F-H2, F-H3, F-H4, F-H5, F-H6, F-H11, F-H15, F-H16, F-H25, F-H30, F-H31, F-H33, F-H55, F-H58, F-H60, F-H65, F-H73, F-H83, F-I1, F-I2, F-I3, F-I4, F-I7, F-I8, F-I11, F-I13, F-I16, F-I17, F-I18, F-I19, F-I22, F-I32, F-J3, F-J26, F-J38, F-J51, F-J59, F-J60, F-J63, F-J65, F-J69, F-J71, F-J72, F-J76, F-J81, F-K9, F-K14, F-K17, F-K20, F-K27, F-K30, F-K33, F-K42, F-K43, F-K45, F-L2, F-L19, F-L26, F-L28, F-L29, F-L30, F-L31, F-L32, F-L40, F-M1, F-M9, F-M12, F-M15, F-M18, F-M19, F-M24, F-A260, F-A263, F-A264, F-A265, F-A266, F-A267, F-A269, F-C234, F-L72, F-L75, F-L76, F-L77, F-L80, F-A270, F-N1, F-N3, F-A279 | F-A1, F-A3, F-A4, F-A5, F-A6, F-A9, F-A10, F-A11, F-A12, F-A13, F-A19, F-A69, F-A73, F-A80, F-A82, F-A85, F-A89, F-A90, F-A91, F-A92, F-A93, F-A94, F-A95, F-A96, F-A97, F-A98, F-A100, F-A105, F-A106, F-A109, F-A112, F-A113, F-A138, F-A154, F-A161, F-A167, F-A179, F-B16, F-B18, F-B43, F-B44, F-B50, F-B56, F-B58, F-B64, F-B66, F-B81, F-B90, F-B93, F-B109, F-C17, F-C33, F-C56, F-C63, F-C71, F-C77, F-C81, F-C95, F-C96, F-C99, F-C144, F-D6, F-D12, F-D15, F-D19, F-D22, F-E30, F-F13, F-G8, F-G12, F-G20, F-G32, F-G34, F-G76, F-H16, F-H26, F-H33, F-H52, F-I3, F-I7, F-I8, F-I11, F-I13, F-I19, F-I32, F-J14, F-J26, F-J38, F-J59, F-J63, F-J65, F-J69, F-J71, F-K27, F-K41, F-K42, F-K43, F-L2, F-L19, F-L28, F-L29, F-L30, F-M9, F-M12, F-M13, F-M15, F-M19, F-A260, F-A262, F-A263, F-A265, F-A266, F-A267, F-A268, F-C234, F-L72, F-L73, F-L75, F-L76, F-L77, F-L79, F-A270, F-N3, F-A280, F-A282, F-A285 | F-A1, F-A3, F-A4, F-A5, F-A6, F-A9, F-A10, F-A11, F-A19, F-A70, F-A72, F-A76, F-A78, F-A80, F-A81, F-A82, F-A86, F-A87, F-A91, F-A93, F-A94, F-A96, F-A98, F-A99, F-A100, F-A103, F-A104, F-A105, F-A106, F-A112, F-A113, F-A137, F-A143, F-A152, F-A157, F-A163, F-A167, F-A169, F-A176, F-A183, F-A191, F-A192, F-A224, F-B16, F-B18, F-B50, F-B56, F-B58, F-B64, F-B66, F-B86, F-B93, F-C33, F-C51, F-C63, F-C71, F-C99, F-C204, F-D6, F-D12, F-D13, F-D15, F-D22, F-E16, F-E18, F-F12, F-G8, F-G10, F-G17, F-G23, F-H15, F-H26, F-H31, F-H33, F-H34, F-H52, F-H58, F-H62, F-I1, F-I2, F-I3, F-I7, F-I8, F-I11, F-I13, F-I17, F-J3, F-J9, F-J14, F-J26, F-J32, F-J38, F-J51, F-J59, F-J65, F-J69, F-J76, F-J81, F-K14, F-K33, F-K38, F-K41, F-K42, F-K43, F-L2, F-L19, F-L28, F-L40, F-L61, F-M9, F-M12, F-M13, F-M15, F-M18, F-M19, F-A260, F-A261, F-A262, F-A263, F-A264, F-A265, F-A266, F-C234, F-L77, F-A270, F-N3 |
| terminal | F-A4, F-A135, F-A160, F-A161, F-A220, F-B2, F-C21, F-C30, F-C32, F-C33, F-C41, F-C77, F-C80, F-C81, F-C143, F-C200, F-C204, F-C206, F-D8, F-D12, F-E10, F-F4, F-F17, F-G2, F-G3, F-G4, F-G5, F-G6, F-G8, F-G9, F-G10, F-G11, F-G12, F-G19, F-G20, F-G23, F-G27, F-G30, F-G31, F-G44, F-G45, F-G49, F-G55, F-H2, F-H11, F-H30, F-H58, F-J3, F-J6, F-J26, F-J43, F-K30, F-K33, F-K34, F-K43, F-L19, F-L28 | F-A4, F-A7, F-A36, F-A160, F-A161, F-B53, F-C21, F-C33, F-C36, F-C42, F-C45, F-C74, F-C75, F-C77, F-C78, F-C80, F-C81, F-C90, F-C144, F-C200, F-C208, F-C210, F-C222, F-D12, F-G2, F-G3, F-G4, F-G5, F-G6, F-G8, F-G9, F-G10, F-G12, F-G14, F-G15, F-G20, F-G21, F-G27, F-G30, F-G31, F-G44, F-G45, F-G46, F-G48, F-G49, F-G52, F-H2, F-H3, F-H4, F-H5, F-H6, F-H47, F-H48, F-H52, F-I15, F-J3, F-J6, F-J9, F-J11, F-J14, F-J26, F-J44, F-K34, F-K43, F-L28, F-L74 | F-A4, F-A17, F-A150, F-B43, F-B53, F-B59, F-C33, F-C36, F-C42, F-C45, F-C51, F-C74, F-C81, F-C204, F-C206, F-D8, F-D12, F-F17, F-F18, F-G3, F-G4, F-G5, F-G6, F-G8, F-G9, F-G10, F-G11, F-G17, F-G21, F-G23, F-G24, F-G27, F-G28, F-G31, F-G44, F-G47, F-G48, F-G49, F-H3, F-H6, F-H18, F-H47, F-H48, F-H52, F-H58, F-J2, F-J3, F-J6, F-J8, F-J9, F-J14, F-J25, F-J26, F-J27, F-J31, F-J44, F-K33, F-K35, F-K43, F-L2, F-L19, F-L28, F-L39, F-L61 |
| script/service | F-A11, F-A113, F-A160, F-A198, F-A250, F-B56, F-B92, F-C23, F-C24, F-C26, F-C27, F-C41, F-C42, F-C45, F-C60, F-C73, F-C74, F-C75, F-C77, F-C199, F-C200, F-C201, F-C202, F-C203, F-C213, F-D7, F-D8, F-E40, F-G20, F-G26, F-G27, F-G29, F-G30, F-G31, F-G34, F-G37, F-G73, F-H10, F-H11, F-H55, F-K34, F-K43, F-L40 | F-A7, F-A113, F-A160, F-A161, F-A198, F-A250, F-B3, F-B56, F-C23, F-C24, F-C26, F-C27, F-C35, F-C39, F-C41, F-C42, F-C45, F-C48, F-C60, F-C73, F-C74, F-C75, F-C84, F-C132, F-C160, F-C164, F-C199, F-C200, F-C201, F-C202, F-C203, F-C213, F-D15, F-E39, F-E40, F-G8, F-G20, F-G26, F-G27, F-G28, F-G29, F-G30, F-G31, F-G68, F-G71, F-H3, F-H41, F-H48, F-H55, F-K24, F-K26, F-K34, F-K43, F-L4, F-L40 | F-A3, F-A7, F-A11, F-A113, F-A136, F-B3, F-B56, F-B92, F-C39, F-C42, F-C45, F-C48, F-C51, F-C74, F-C75, F-C158, F-D7, F-D8, F-D15, F-E40, F-G2, F-G26, F-G27, F-G28, F-G29, F-G30, F-G31, F-G37, F-G71, F-G73, F-H3, F-H35, F-H48, F-H55, F-I19, F-K26, F-K43, F-L40, F-M5, F-L76 |
| label | F-A1, F-A3, F-A5, F-A6, F-A19, F-A160, F-A161, F-A167, F-C53, F-C54, F-C55, F-C56, F-C57, F-C72, F-C99, F-C173, F-C174, F-C176, F-C177, F-C178, F-C179, F-C196, F-D15, F-D19, F-E10, F-E21, F-E27, F-E35, F-G8, F-G23, F-G31, F-I2, F-I13, F-J38, F-J72, F-K8, F-K14, F-K17, F-K19, F-K23, F-K43, F-K46, F-A269, F-C235, F-A270 | F-A3, F-A6, F-A10, F-A14, F-A15, F-A16, F-A19, F-A167, F-C36, F-C52, F-C53, F-C54, F-C55, F-C56, F-C57, F-C72, F-C81, F-C99, F-C173, F-C174, F-C175, F-C176, F-C177, F-C178, F-C179, F-C196, F-D15, F-D19, F-E10, F-E21, F-E35, F-I13, F-K14, F-K19, F-K23, F-K24, F-K43, F-M13, F-A269, F-C235, F-A270 | F-A1, F-A3, F-A5, F-A6, F-A10, F-A14, F-A19, F-A160, F-C57, F-C72, F-C99, F-D15, F-D19, F-E21, F-E35, F-G31, F-J38, F-K14, F-K19, F-K23, F-K43, F-M13, F-A269, F-C235 |
| schedule | F-A3, F-A5, F-A161, F-A174, F-A216, F-A220, F-D6, F-D7, F-D8, F-D15, F-H3, F-H7, F-H16, F-H61, F-H65, F-I1, F-I2, F-I3, F-I4, F-I5, F-I6, F-I7, F-I8, F-I11, F-I22, F-I27, F-I30, F-I32, F-I34, F-J3, F-J6, F-K9 | F-A3, F-A5, F-A99, F-A174, F-B12, F-B74, F-C41, F-D6, F-D7, F-D8, F-E3, F-F13, F-H3, F-H61, F-I1, F-I2, F-I3, F-I5, F-I6, F-I7, F-I8, F-I9, F-I10, F-I13, F-I16, F-I23, F-I25, F-I27, F-I31, F-I32, F-I34, F-I35, F-J9, F-J33, F-J44, F-N4 | F-A3, F-A17, F-A157, F-A174, F-B12, F-D6, F-D7, F-D8, F-F18, F-H3, F-H18, F-H61, F-I1, F-I2, F-I3, F-I5, F-I6, F-I7, F-I8, F-I9, F-I13, F-I25, F-I28, F-I30, F-J9, F-J44, F-K8, F-J102, F-N4 |
| plugin | F-A3, F-A10, F-A70, F-A84, F-A85, F-A94, F-A101, F-A113, F-A167, F-A169, F-A205, F-B25, F-B47, F-B57, F-B70, F-B71, F-B81, F-C77, F-C204, F-E5, F-E27, F-F12, F-F13, F-F33, F-G20, F-H11, F-H30, F-I11, F-I40, F-J51, F-J59, F-J63, F-J69, F-K2, F-K9, F-K30, F-K42, F-K43, F-K46, F-L1, F-L9, F-L19, F-M1, F-M3, F-M6, F-M8, F-M9, F-M12, F-M16, F-M17, F-M18, F-M19, F-M21, F-M24, F-A267, F-L72, F-L73, F-L75 | F-A3, F-A6, F-A9, F-A10, F-A112, F-A113, F-A153, F-A193, F-B47, F-B50, F-B71, F-B81, F-C35, F-C36, F-C39, F-C42, F-C77, F-C149, F-E39, F-E79, F-F13, F-F32, F-G6, F-G20, F-I47, F-J69, F-K41, F-L19, F-M1, F-M3, F-M6, F-M7, F-M8, F-M9, F-M12, F-M13, F-M17, F-M19, F-L72, F-L73 | F-A3, F-A6, F-A9, F-A10, F-A70, F-A93, F-A94, F-A101, F-A112, F-A113, F-A134, F-A152, F-A153, F-A169, F-B18, F-B47, F-B50, F-B75, F-C35, F-C36, F-C39, F-C42, F-C149, F-C204, F-E39, F-E79, F-F11, F-F12, F-F18, F-G3, F-G6, F-H30, F-I40, F-J33, F-J51, F-J69, F-K9, F-K30, F-K41, F-K42, F-K43, F-L2, F-L19, F-M1, F-M7, F-M8, F-M9, F-M12, F-M13, F-M14, F-M17, F-M18, F-M19 |

## Cross-area links

Each arrow is a dependency/reference between the named feature surfaces, not a claim of causal direction beyond the quoted survey explanation. Repeated directed ID pairs are merged and distinct survey explanations retained. Every Cross-area connections entry from all 17 survey files is represented; same-area arrows are retained when two distinct features share that area letter.

F-A2 -> F-A139: Timeline/stream (agent_stream, timeline_replacement, hydrateTimelineFromProvider, durable timeline store) <- all lifecycle ops. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A7 -> F-C13: Workspaces/projects/worktrees: create (`workspace.create.request`), archive scope, auto-archive on merge, background workspaces, naming (`workspaceAutoName`), `workspaceId` stamping/backfill. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A7 -> F-E20: Workspaces/projects/worktrees: create (`workspace.create.request`), archive scope, auto-archive on merge, background workspaces, naming (`workspaceAutoName`), `workspaceId` stamping/backfill. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A6 -> F-A100: Providers/models/modes/features: `resolveCreateConfig`, `AgentClient.createSession/resumeSession/importSession/listImportableSessions`, capabilities (`supportsSessionListing`, `supportsRewind*`, `canStopRuntime`), native archive sync. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A17 -> F-A262: Permissions/questions: attention `permission`, `needs_input` bucket, idle cleanup veto. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A17 -> F-J36: Permissions/questions: attention `permission`, `needs_input` bucket, idle cleanup veto. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A9 -> F-A157: Prompts/messages: `startCreatedAgentInitialPrompt`, `sendPromptToAgent` (unarchive, resume prompt), `clientMessageId`, `agent-messages` delivery protection in idle cleanup. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A9 -> F-B26: Prompts/messages: `startCreatedAgentInitialPrompt`, `sendPromptToAgent` (unarchive, resume prompt), `clientMessageId`, `agent-messages` delivery protection in idle cleanup. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A10 -> F-A106: Subagents/MCP: parent label cascade, `create_agent` MCP tool (`createAgentCommand kind "mcp"`), finish subscriptions, provider subagents. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A10 -> F-C53: Subagents/MCP: parent label cascade, `create_agent` MCP tool (`createAgentCommand kind "mcp"`), finish subscriptions, provider subagents. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A10 -> F-I12: Subagents/MCP: parent label cascade, `create_agent` MCP tool (`createAgentCommand kind "mcp"`), finish subscriptions, provider subagents. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A3 -> F-H59: Hub: `hub.execution.agent.create|validate.request` (owner `{kind:"daemon"}`; `validateAgentConfigurationAgainstProvider`), `agent-owner`. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A3 -> F-N3: Hub: `hub.execution.agent.create|validate.request` (owner `{kind:"daemon"}`; `validateAgentConfigurationAgainstProvider`), `agent-owner`. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A17 -> F-H49: Notifications/push/desktop: attention fan-out, `client_heartbeat`, push tokens, deep links, desktop argv handler. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A17 -> F-I1: Notifications/push/desktop: attention fan-out, `client_heartbeat`, push tokens, deep links, desktop argv handler. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A17 -> F-J30: Notifications/push/desktop: attention fan-out, `client_heartbeat`, push tokens, deep links, desktop argv handler. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A1 -> F-H59: Settings/daemon config: `agents.resumeOnStartup`, `agents.resumePrompt`, `agents.idleRuntimeCleanup`, `daemon.autoArchiveAfterMerge`. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A1 -> F-L19: Settings/daemon config: `agents.resumeOnStartup`, `agents.resumePrompt`, `agents.idleRuntimeCleanup`, `daemon.autoArchiveAfterMerge`. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A10 -> F-A111: Plugins: lifecycle hooks `agent.create.before/after`, `agent.archive.after`; plugin provider session history. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A10 -> F-M13: Plugins: lifecycle hooks `agent.create.before/after`, `agent.archive.after`; plugin provider session history. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A3 -> F-I16: Schedules: scheduled runs create/resume agents. [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A5 -> F-A133: CLI/client/relay: all requests go through `DaemonClient` (WebSocket or relay E2EE); features negotiated via `status.server_info.features` (`creationLifecycle`, `agentRequestReceipts`, `workspaceRequestReceipts`, `agentForkContext`, `idleRuntimeCleanupSetting`, `resumeOnStartupSetting`, `workspaceMultiplicity`, `forgeSearch`, `wholeTurnTimeline`). [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A5 -> F-C84: CLI/client/relay: all requests go through `DaemonClient` (WebSocket or relay E2EE); features negotiated via `status.server_info.features` (`creationLifecycle`, `agentRequestReceipts`, `workspaceRequestReceipts`, `agentForkContext`, `idleRuntimeCleanupSetting`, `resumeOnStartupSetting`, `workspaceMultiplicity`, `forgeSearch`, `wholeTurnTimeline`). [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A5 -> F-E53: CLI/client/relay: all requests go through `DaemonClient` (WebSocket or relay E2EE); features negotiated via `status.server_info.features` (`creationLifecycle`, `agentRequestReceipts`, `workspaceRequestReceipts`, `agentForkContext`, `idleRuntimeCleanupSetting`, `resumeOnStartupSetting`, `workspaceMultiplicity`, `forgeSearch`, `wholeTurnTimeline`). [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A5 -> F-H19: CLI/client/relay: all requests go through `DaemonClient` (WebSocket or relay E2EE); features negotiated via `status.server_info.features` (`creationLifecycle`, `agentRequestReceipts`, `workspaceRequestReceipts`, `agentForkContext`, `idleRuntimeCleanupSetting`, `resumeOnStartupSetting`, `workspaceMultiplicity`, `forgeSearch`, `wholeTurnTimeline`). [Survey: parts/A-lifecycle.md#cross-area-connections]

F-A79 -> F-A2: Agent lifecycle and creation use provider/model/mode discovery, resolved launch configuration and capability validation; config changes and plugin registrations rebuild provider registry/catalog state. [Survey: parts/B-providers.md#cross-area-connections]

F-A79 -> F-L19: Agent lifecycle and creation use provider/model/mode discovery, resolved launch configuration and capability validation; config changes and plugin registrations rebuild provider registry/catalog state. [Survey: parts/B-providers.md#cross-area-connections]

F-A79 -> F-M20: Agent lifecycle and creation use provider/model/mode discovery, resolved launch configuration and capability validation; config changes and plugin registrations rebuild provider registry/catalog state. [Survey: parts/B-providers.md#cross-area-connections]

F-A79 -> F-A10: Provider sessions emit timeline, usage and permission/question control events; plan responses can launch implementation turns. Provider modes determine unattended behavior and exact MCP preapproval. [Survey: parts/B-providers.md#cross-area-connections]

F-A79 -> F-A133: Provider sessions emit timeline, usage and permission/question control events; plan responses can launch implementation turns. Provider modes determine unattended behavior and exact MCP preapproval. [Survey: parts/B-providers.md#cross-area-connections]

F-A79 -> F-A260: Provider sessions emit timeline, usage and permission/question control events; plan responses can launch implementation turns. Provider modes determine unattended behavior and exact MCP preapproval. [Survey: parts/B-providers.md#cross-area-connections]

F-A79 -> F-I11: Provider sessions emit timeline, usage and permission/question control events; plan responses can launch implementation turns. Provider modes determine unattended behavior and exact MCP preapproval. [Survey: parts/B-providers.md#cross-area-connections]

F-A79 -> F-J80: Provider sessions emit timeline, usage and permission/question control events; plan responses can launch implementation turns. Provider modes determine unattended behavior and exact MCP preapproval. [Survey: parts/B-providers.md#cross-area-connections]

F-A90 -> F-A12: Composer consumes provider commands and skills; importable sessions feed history/import/resume; persistence handles and native archive/rewind capabilities connect to lifecycle and timeline replacement. [Survey: parts/B-providers.md#cross-area-connections]

F-A90 -> F-A166: Composer consumes provider commands and skills; importable sessions feed history/import/resume; persistence handles and native archive/rewind capabilities connect to lifecycle and timeline replacement. [Survey: parts/B-providers.md#cross-area-connections]

F-A90 -> F-B42: Composer consumes provider commands and skills; importable sessions feed history/import/resume; persistence handles and native archive/rewind capabilities connect to lifecycle and timeline replacement. [Survey: parts/B-providers.md#cross-area-connections]

F-A90 -> F-H62: Composer consumes provider commands and skills; importable sessions feed history/import/resume; persistence handles and native archive/rewind capabilities connect to lifecycle and timeline replacement. [Survey: parts/B-providers.md#cross-area-connections]

F-A90 -> F-I19: Composer consumes provider commands and skills; importable sessions feed history/import/resume; persistence handles and native archive/rewind capabilities connect to lifecycle and timeline replacement. [Survey: parts/B-providers.md#cross-area-connections]

F-A67 -> F-A14: Provider subagent/subsession adoption connects parentage, workspace placement and timeline cards. Host provider settings expose discovery errors, model availability and diagnostics. [Survey: parts/B-providers.md#cross-area-connections]

F-A67 -> F-A133: Provider subagent/subsession adoption connects parentage, workspace placement and timeline cards. Host provider settings expose discovery errors, model availability and diagnostics. [Survey: parts/B-providers.md#cross-area-connections]

F-A67 -> F-C68: Provider subagent/subsession adoption connects parentage, workspace placement and timeline cards. Host provider settings expose discovery errors, model availability and diagnostics. [Survey: parts/B-providers.md#cross-area-connections]

F-A67 -> F-H29: Provider subagent/subsession adoption connects parentage, workspace placement and timeline cards. Host provider settings expose discovery errors, model availability and diagnostics. [Survey: parts/B-providers.md#cross-area-connections]

F-A67 -> F-L19: Provider subagent/subsession adoption connects parentage, workspace placement and timeline cards. Host provider settings expose discovery errors, model availability and diagnostics. [Survey: parts/B-providers.md#cross-area-connections]

F-A82 -> F-H29: Plugins negotiate provider capabilities and subprocess messages; host config controls enablement/env/options/catalog overrides. Missing profile CRUD/application is in ../gaps.md; bundled orchestration skill management is in ../.area6-parts/A.md. [Survey: parts/B-providers.md#cross-area-connections]

F-A82 -> F-I19: Plugins negotiate provider capabilities and subprocess messages; host config controls enablement/env/options/catalog overrides. Missing profile CRUD/application is in ../gaps.md; bundled orchestration skill management is in ../.area6-parts/A.md. [Survey: parts/B-providers.md#cross-area-connections]

F-A82 -> F-L19: Plugins negotiate provider capabilities and subprocess messages; host config controls enablement/env/options/catalog overrides. Missing profile CRUD/application is in ../gaps.md; bundled orchestration skill management is in ../.area6-parts/A.md. [Survey: parts/B-providers.md#cross-area-connections]

F-A82 -> F-M21: Plugins negotiate provider capabilities and subprocess messages; host config controls enablement/env/options/catalog overrides. Missing profile CRUD/application is in ../gaps.md; bundled orchestration skill management is in ../.area6-parts/A.md. [Survey: parts/B-providers.md#cross-area-connections]

F-A173 -> F-B42: Composer / message submission (`A/composer/submission`, `pendingMessageSubmissions`, `getSendingClientMessageIds`): optimistic user rows carry `clientMessageId`; server `recordSubmittedPrompt`/`reconcileSubmittedPromptEcho` dedupe; `message-sent` bottom-anchor reason. [Survey: parts/C-timeline.md#cross-area-connections]

F-A143 -> F-A260: Permissions (`PermissionRequestCard`, `agent_permission_request`, `permission_requested/resolved` events): rendered at the foot of the stream; `plan_approval`/`ExitPlanMode` tool rows hidden while pending then shown as `PlanCard`; `resolvePendingPermissionsForAgent` on turn failure/cancel. [Survey: parts/C-timeline.md#cross-area-connections]

F-A190 -> F-A2: Agent lifecycle/status (`agent_status`, `AgentSnapshotPayload.activeTurn`, `lastUsage`, `runtimeInfo`): snapshot returned inside every `fetch_agent_timeline_response.agent`; drives turn liveness, working footer, header usage meter. [Survey: parts/C-timeline.md#cross-area-connections]

F-A190 -> F-J80: Agent lifecycle/status (`agent_status`, `AgentSnapshotPayload.activeTurn`, `lastUsage`, `runtimeInfo`): snapshot returned inside every `fetch_agent_timeline_response.agent`; drives turn liveness, working footer, header usage meter. [Survey: parts/C-timeline.md#cross-area-connections]

F-A157 -> F-A12: Rewind/fork (`agent.rewind.request/response`, `agent.fork_context.*`, `useForkAgent`, `useRewindAgentMutation`): consume timeline cursors/messageIds; rewind causes `timeline_replacement` + new epoch. [Survey: parts/C-timeline.md#cross-area-connections]

F-A157 -> F-A3: Sub-agents (`A/subagents`, `provider-subagents/store.ts` which reuses `InMemoryAgentTimelineStore`, `agent.provider_subagents.*`): `sub_agent` detail is a launch marker; their own projected timeline paging uses the same cursor/epoch protocol. / Plugins (`A/plugins/timeline`, `agent.timeline.append.request`, plugin timeline items, `useInstalledTimelineTransform`): server-stored plugin rows and client-side transforms. [Survey: parts/C-timeline.md#cross-area-connections]

F-A157 -> F-A79: Sub-agents (`A/subagents`, `provider-subagents/store.ts` which reuses `InMemoryAgentTimelineStore`, `agent.provider_subagents.*`): `sub_agent` detail is a launch marker; their own projected timeline paging uses the same cursor/epoch protocol. [Survey: parts/C-timeline.md#cross-area-connections]

F-A157 -> F-C19: Sub-agents (`A/subagents`, `provider-subagents/store.ts` which reuses `InMemoryAgentTimelineStore`, `agent.provider_subagents.*`): `sub_agent` detail is a launch marker; their own projected timeline paging uses the same cursor/epoch protocol. [Survey: parts/C-timeline.md#cross-area-connections]

F-A165 -> F-J36: Attention/notifications (`attention_required`, `agent_attention_required`): split off the timeline stream per client. [Survey: parts/C-timeline.md#cross-area-connections]

F-A157 -> F-M21: Plugins (`A/plugins/timeline`, `agent.timeline.append.request`, plugin timeline items, `useInstalledTimelineTransform`): server-stored plugin rows and client-side transforms. [Survey: parts/C-timeline.md#cross-area-connections]

F-A159 -> F-C83: MCP / CLI (`get_agent_activity`, `wait`/`logs` commands): use `activity-curator` and `selectItemsByProjectedLimit`; CLI uses `subscribeAgentTimeline` / `fetchAgentTimeline`. [Survey: parts/C-timeline.md#cross-area-connections]

F-A159 -> F-I11: MCP / CLI (`get_agent_activity`, `wait`/`logs` commands): use `activity-curator` and `selectItemsByProjectedLimit`; CLI uses `subscribeAgentTimeline` / `fetchAgentTimeline`. [Survey: parts/C-timeline.md#cross-area-connections]

F-A133 -> F-C68: Workspace/git: `commandMayHaveChangedExternalState` on completed shell calls triggers workspace git refresh. [Survey: parts/C-timeline.md#cross-area-connections]

F-A133 -> F-D8: Workspace/git: `commandMayHaveChangedExternalState` on completed shell calls triggers workspace git refresh. [Survey: parts/C-timeline.md#cross-area-connections]

F-A142 -> F-L74: Settings: `chatDisplay`, `autoExpandReasoning`, `chatOutlineEnabled`, content max width. [Survey: parts/C-timeline.md#cross-area-connections]

F-A189 -> F-A10: Voice: `speak` tool rows and `<spoken-input>` envelope; voice mode auto-allows speak permission (`session.ts:2043`). [Survey: parts/C-timeline.md#cross-area-connections]

F-A189 -> F-A77: Voice: `speak` tool rows and `<spoken-input>` envelope; voice mode auto-allows speak permission (`session.ts:2043`). [Survey: parts/C-timeline.md#cross-area-connections]

F-A189 -> F-A260: Voice: `speak` tool rows and `<spoken-input>` envelope; voice mode auto-allows speak permission (`session.ts:2043`). [Survey: parts/C-timeline.md#cross-area-connections]

F-A189 -> F-B90: Voice: `speak` tool rows and `<spoken-input>` envelope; voice mode auto-allows speak permission (`session.ts:2043`). [Survey: parts/C-timeline.md#cross-area-connections]

F-A143 -> F-I16: Schedules/heartbeats: `<paseo-system>` envelope → `heartbeat` tool-call rows (turn triggers). [Survey: parts/C-timeline.md#cross-area-connections]

F-A136 -> F-A3: Providers: all tool-call mapping per provider (`S/agent/providers/*/tool-call-mapper*`) produce the ToolCallDetail kinds; provider history hydration feeds the timeline with `fromHistory` events. [Survey: parts/C-timeline.md#cross-area-connections]

F-A136 -> F-A100: Providers: all tool-call mapping per provider (`S/agent/providers/*/tool-call-mapper*`) produce the ToolCallDetail kinds; provider history hydration feeds the timeline with `fromHistory` events. [Survey: parts/C-timeline.md#cross-area-connections]

F-A141 -> F-A3: Host runtime / directory sync / replica cache (`A/runtime`): fetch dedup, agent snapshot merge, cached painting. [Survey: parts/C-timeline.md#cross-area-connections]

F-A141 -> F-C98: Host runtime / directory sync / replica cache (`A/runtime`): fetch dedup, agent snapshot merge, cached painting. [Survey: parts/C-timeline.md#cross-area-connections]

F-A141 -> F-E20: Host runtime / directory sync / replica cache (`A/runtime`): fetch dedup, agent snapshot merge, cached painting. [Survey: parts/C-timeline.md#cross-area-connections]

F-A141 -> F-H62: Host runtime / directory sync / replica cache (`A/runtime`): fetch dedup, agent snapshot merge, cached painting. [Survey: parts/C-timeline.md#cross-area-connections]

F-A141 -> F-L79: Host runtime / directory sync / replica cache (`A/runtime`): fetch dedup, agent snapshot merge, cached painting. [Survey: parts/C-timeline.md#cross-area-connections]

F-B7 -> F-A3: Composer editing and local drafts feed workspace/agent creation; creation handoff transfers optimistic message identity to the new agent. [Survey: parts/D-messaging.md#cross-area-connections]

F-B7 -> F-A157: Composer editing and local drafts feed workspace/agent creation; creation handoff transfers optimistic message identity to the new agent. [Survey: parts/D-messaging.md#cross-area-connections]

F-B7 -> F-C34: Composer editing and local drafts feed workspace/agent creation; creation handoff transfers optimistic message identity to the new agent. [Survey: parts/D-messaging.md#cross-area-connections]

F-B17 -> F-A155: Client message ids and server receipts connect optimistic timeline reconciliation, reconnect retries, idempotency and authoritative prompt dispatch. [Survey: parts/D-messaging.md#cross-area-connections]

F-B17 -> F-H1: Client message ids and server receipts connect optimistic timeline reconciliation, reconnect retries, idempotency and authoritative prompt dispatch. [Survey: parts/D-messaging.md#cross-area-connections]

F-B19 -> F-A2: Steer/interrupt capability, cancellation and two distinct message queues depend on provider runtime and agent run lifecycle. [Survey: parts/D-messaging.md#cross-area-connections]

F-B19 -> F-A88: Steer/interrupt capability, cancellation and two distinct message queues depend on provider runtime and agent run lifecycle. [Survey: parts/D-messaging.md#cross-area-connections]

F-B19 -> F-H62: Steer/interrupt capability, cancellation and two distinct message queues depend on provider runtime and agent run lifecycle. [Survey: parts/D-messaging.md#cross-area-connections]

F-B23 -> F-A11: Attachments join image storage, uploaded files, workspace selections, fork history, forge/review context, browser elements and plugin resources through the common prompt builder. [Survey: parts/D-messaging.md#cross-area-connections]

F-B23 -> F-C68: Attachments join image storage, uploaded files, workspace selections, fork history, forge/review context, browser elements and plugin resources through the common prompt builder. [Survey: parts/D-messaging.md#cross-area-connections]

F-B23 -> F-E1: Attachments join image storage, uploaded files, workspace selections, fork history, forge/review context, browser elements and plugin resources through the common prompt builder. [Survey: parts/D-messaging.md#cross-area-connections]

F-B23 -> F-F7: Attachments join image storage, uploaded files, workspace selections, fork history, forge/review context, browser elements and plugin resources through the common prompt builder. [Survey: parts/D-messaging.md#cross-area-connections]

F-B23 -> F-G36: Attachments join image storage, uploaded files, workspace selections, fork history, forge/review context, browser elements and plugin resources through the common prompt builder. [Survey: parts/D-messaging.md#cross-area-connections]

F-B23 -> F-M20: Attachments join image storage, uploaded files, workspace selections, fork history, forge/review context, browser elements and plugin resources through the common prompt builder. [Survey: parts/D-messaging.md#cross-area-connections]

F-B26 -> F-A3: Prompt-source envelopes preserve human/voice/MCP/inter-agent sender identity; new human prompts can clear pending permissions. Missing approval RPC/card details are in ../gaps.md. [Survey: parts/D-messaging.md#cross-area-connections]

F-B26 -> F-A260: Prompt-source envelopes preserve human/voice/MCP/inter-agent sender identity; new human prompts can clear pending permissions. Missing approval RPC/card details are in ../gaps.md. [Survey: parts/D-messaging.md#cross-area-connections]

F-B26 -> F-I11: Prompt-source envelopes preserve human/voice/MCP/inter-agent sender identity; new human prompts can clear pending permissions. Missing approval RPC/card details are in ../gaps.md. [Survey: parts/D-messaging.md#cross-area-connections]

F-B27 -> F-A10: Structured-response helpers use temporary internal agents and provider fallback for workspace naming and git commit/PR metadata; they are separate from the user chat loop. [Survey: parts/D-messaging.md#cross-area-connections]

F-B27 -> F-A106: Structured-response helpers use temporary internal agents and provider fallback for workspace naming and git commit/PR metadata; they are separate from the user chat loop. [Survey: parts/D-messaging.md#cross-area-connections]

F-B27 -> F-C68: Structured-response helpers use temporary internal agents and provider fallback for workspace naming and git commit/PR metadata; they are separate from the user chat loop. [Survey: parts/D-messaging.md#cross-area-connections]

F-B27 -> F-D14: Structured-response helpers use temporary internal agents and provider fallback for workspace naming and git commit/PR metadata; they are separate from the user chat loop. [Survey: parts/D-messaging.md#cross-area-connections]

F-B27 -> F-E29: Structured-response helpers use temporary internal agents and provider fallback for workspace naming and git commit/PR metadata; they are separate from the user chat loop. [Survey: parts/D-messaging.md#cross-area-connections]

F-B50 -> F-A2: Agent lifecycle/turns: `send_agent_message_request`, `cancel_agent_request`, `startAgentRun`/steer/replace, `agent.interrupt`, pending permissions (`pendingPermissions` in session store), unarchive-on-send, provisional titles on first message, `clearPendingPermissions`. [Survey: area-2-composer-input.md#cross-area-connections]

F-B50 -> F-A269: Agent lifecycle/turns: `send_agent_message_request`, `cancel_agent_request`, `startAgentRun`/steer/replace, `agent.interrupt`, pending permissions (`pendingPermissions` in session store), unarchive-on-send, provisional titles on first message, `clearPendingPermissions`. [Survey: area-2-composer-input.md#cross-area-connections]

F-B52 -> F-A139: Timeline/stream: optimistic user rows, `messageSubmissions`, canonical `user_message` reconciliation (`acknowledgedClientMessageIds`), recall reads the stream, turn liveness (`selectAgentTurnPresentation`, `beginAgentCancellation`), queue drain triggers from timeline sync and `onAgentStoppedRunning`. [Survey: area-2-composer-input.md#cross-area-connections]

F-B76 -> F-A3: Agent creation / new workspace flow: draft tabs (`useAgentInputDraft`, `useAgentFormState`, create-flow, `handoffCreatedAgentMessageSubmission`), `commandDraft` for slash-command listing before an agent exists, `agent.create`/`workspace.create` requests with `attachments`/`firstAgentContext`. [Survey: area-2-composer-input.md#cross-area-connections]

F-B76 -> F-C34: Agent creation / new workspace flow: draft tabs (`useAgentInputDraft`, `useAgentFormState`, create-flow, `handoffCreatedAgentMessageSubmission`), `commandDraft` for slash-command listing before an agent exists, `agent.create`/`workspace.create` requests with `attachments`/`firstAgentContext`. [Survey: area-2-composer-input.md#cross-area-connections]

F-B42 -> F-A3: Agent controls (mode/model/thinking/features, Shift+Tab cycle) share the composer footer. [Survey: area-2-composer-input.md#cross-area-connections]

F-B42 -> F-A98: Agent controls (mode/model/thinking/features, Shift+Tab cycle) share the composer footer. [Survey: area-2-composer-input.md#cross-area-connections]

F-B56 -> F-C86: Workspace tabs / layout: `resolveFocusedChatTarget`, `revealWorkspaceTab`, `closeWorkspaceTab`, `retargetCurrentTab`, `unpinWorkspaceAgent/hideWorkspaceAgent` (client slash commands), `archiveAgent` RPC. [Survey: area-2-composer-input.md#cross-area-connections]

F-B87 -> F-C68: File explorer / files panel / diff panel / git: "Add to chat", workspace-file drag source, `getDirectorySuggestions` (@ mentions and file links), `readFile` binary file transfer (assistant images, file previews), `listDirectory` (path opener), file downloads (`file_download_token_request`, `/api/files/download?token=` single-use token, 60 s TTL `downloadTokenTtlMs`, owned by the explorer download action). [Survey: area-2-composer-input.md#cross-area-connections]

F-B87 -> F-D22: File explorer / files panel / diff panel / git: "Add to chat", workspace-file drag source, `getDirectorySuggestions` (@ mentions and file links), `readFile` binary file transfer (assistant images, file previews), `listDirectory` (path opener), file downloads (`file_download_token_request`, `/api/files/download?token=` single-use token, 60 s TTL `downloadTokenTtlMs`, owned by the explorer download action). [Survey: area-2-composer-input.md#cross-area-connections]

F-B87 -> F-F3: File explorer / files panel / diff panel / git: "Add to chat", workspace-file drag source, `getDirectorySuggestions` (@ mentions and file links), `readFile` binary file transfer (assistant images, file previews), `listDirectory` (path opener), file downloads (`file_download_token_request`, `/api/files/download?token=` single-use token, 60 s TTL `downloadTokenTtlMs`, owned by the explorer download action). [Survey: area-2-composer-input.md#cross-area-connections]

F-B83 -> F-D12: Forge/GitHub: forge search RPC and checkout PR status for the picker and auto-attach; `serverInfo.features.forgeSearch` legacy fallback. [Survey: area-2-composer-input.md#cross-area-connections]

F-B83 -> F-E10: Forge/GitHub: forge search RPC and checkout PR status for the picker and auto-attach; `serverInfo.features.forgeSearch` legacy fallback. [Survey: area-2-composer-input.md#cross-area-connections]

F-B80 -> F-A11: Browser pane / review / fork-context ("chat history"): produce workspace attachments consumed on send. [Survey: area-2-composer-input.md#cross-area-connections]

F-B80 -> F-C90: Browser pane / review / fork-context ("chat history"): produce workspace attachments consumed on send. [Survey: area-2-composer-input.md#cross-area-connections]

F-B80 -> F-E49: Browser pane / review / fork-context ("chat history"): produce workspace attachments consumed on send. [Survey: area-2-composer-input.md#cross-area-connections]

F-B80 -> F-G36: Browser pane / review / fork-context ("chat history"): produce workspace attachments consumed on send. / Desktop (Electron): attachment file store, native dialogs, `readDesktopFileBytes`, desktop host bridge (`getDesktopHost`, `isElectronRuntime`), browser-webview element picker. [Survey: area-2-composer-input.md#cross-area-connections]

F-B57 -> F-K40: Plugins: composer pills/attachment sources, client slash commands, theme/audio RPC (`createPlayAudio`), command center. [Survey: area-2-composer-input.md#cross-area-connections]

F-B57 -> F-L11: Plugins: composer pills/attachment sources, client slash commands, theme/audio RPC (`createPlayAudio`), command center. [Survey: area-2-composer-input.md#cross-area-connections]

F-B57 -> F-M21: Plugins: composer pills/attachment sources, client slash commands, theme/audio RPC (`createPlayAudio`), command center. [Survey: area-2-composer-input.md#cross-area-connections]

F-B90 -> F-K38: Settings: `sendBehavior`, `sendShortcut`, `openInSidePane`, `filePanelAssociations`, voice/dictation settings page, keyboard shortcut overrides (`shortcut-override-store`), language (i18n keys `composer.*`, `message.dictation.*`, `realtimeVoice.*`, `settings.voice.*`). [Survey: area-2-composer-input.md#cross-area-connections]

F-B90 -> F-L75: Settings: `sendBehavior`, `sendShortcut`, `openInSidePane`, `filePanelAssociations`, voice/dictation settings page, keyboard shortcut overrides (`shortcut-override-store`), language (i18n keys `composer.*`, `message.dictation.*`, `realtimeVoice.*`, `settings.voice.*`). [Survey: area-2-composer-input.md#cross-area-connections]

F-B66 -> F-A9: Daemon config/auth: `patchDaemonConfig`, `get_daemon_config`, operation permissions (`daemon.read|manage`), `server_info` capabilities and `features` flags, host connection state (`useHostRuntimeIsConnected`, `subscribeConnectionStatus` for dictation resume). [Survey: area-2-composer-input.md#cross-area-connections]

F-B66 -> F-A265: Daemon config/auth: `patchDaemonConfig`, `get_daemon_config`, operation permissions (`daemon.read|manage`), `server_info` capabilities and `features` flags, host connection state (`useHostRuntimeIsConnected`, `subscribeConnectionStatus` for dictation resume). [Survey: area-2-composer-input.md#cross-area-connections]

F-B66 -> F-H36: Daemon config/auth: `patchDaemonConfig`, `get_daemon_config`, operation permissions (`daemon.read|manage`), `server_info` capabilities and `features` flags, host connection state (`useHostRuntimeIsConnected`, `subscribeConnectionStatus` for dictation resume). [Survey: area-2-composer-input.md#cross-area-connections]

F-B66 -> F-L19: Daemon config/auth: `patchDaemonConfig`, `get_daemon_config`, operation permissions (`daemon.read|manage`), `server_info` capabilities and `features` flags, host connection state (`useHostRuntimeIsConnected`, `subscribeConnectionStatus` for dictation resume). [Survey: area-2-composer-input.md#cross-area-connections]

F-B80 -> F-F12: Desktop (Electron): attachment file store, native dialogs, `readDesktopFileBytes`, desktop host bridge (`getDesktopHost`, `isElectronRuntime`), browser-webview element picker. [Survey: area-2-composer-input.md#cross-area-connections]

F-B80 -> F-H49: Desktop (Electron): attachment file store, native dialogs, `readDesktopFileBytes`, desktop host bridge (`getDesktopHost`, `isElectronRuntime`), browser-webview element picker. [Survey: area-2-composer-input.md#cross-area-connections]

F-B90 -> F-A3: Notifications/attention: `onAttentionInputFocus`, `onAttentionPromptSend` (clears agent attention when the user types/sends), `activity_log` voice abort toast. [Survey: area-2-composer-input.md#cross-area-connections]

F-B90 -> F-J36: Notifications/attention: `onAttentionInputFocus`, `onAttentionPromptSend` (clears agent attention when the user types/sends), `activity_log` voice abort toast. [Survey: area-2-composer-input.md#cross-area-connections]

F-B87 -> F-A167: Markdown renderer / chat view: assistant selection copy surface, markdown images, file links, lightbox. [Survey: area-2-composer-input.md#cross-area-connections]

F-B87 -> F-F13: Markdown renderer / chat view: assistant selection copy surface, markdown images, file links, lightbox. [Survey: area-2-composer-input.md#cross-area-connections]

F-B94 -> F-A77: Terminal: composer "terminal" mode and terminal find. [Survey: area-2-composer-input.md#cross-area-connections]

F-B94 -> F-G11: Terminal: composer "terminal" mode and terminal find. [Survey: area-2-composer-input.md#cross-area-connections]

F-C52 -> F-A2: **Agents/sessions**: workspace id is ownership for agent create/import/archive, terminal ownership and activity aggregation; combined create carries provider settings and first message. Closing root-agent tabs archives; child-agent tabs can only change client-open labels. [A-lifecycle.md](A-lifecycle.md), [B-providers.md](B-providers.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C52 -> F-A100: **Agents/sessions**: workspace id is ownership for agent create/import/archive, terminal ownership and activity aggregation; combined create carries provider settings and first message. Closing root-agent tabs archives; child-agent tabs can only change client-open labels. [A-lifecycle.md](A-lifecycle.md), [B-providers.md](B-providers.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C52 -> F-G12: **Agents/sessions**: workspace id is ownership for agent create/import/archive, terminal ownership and activity aggregation; combined create carries provider settings and first message. Closing root-agent tabs archives; child-agent tabs can only change client-open labels. [A-lifecycle.md](A-lifecycle.md), [B-providers.md](B-providers.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C52 -> F-L7: **Agents/sessions**: workspace id is ownership for agent create/import/archive, terminal ownership and activity aggregation; combined create carries provider settings and first message. Closing root-agent tabs archives; child-agent tabs can only change client-open labels. [A-lifecycle.md](A-lifecycle.md), [B-providers.md](B-providers.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C95 -> F-A11: **Timeline/composer/attachments**: draft-to-created-workspace handoff, fork history attachment, file/image review context, provisional title seed, pending initial create overlay and attention/read state. [C-timeline.md](C-timeline.md), [D-messaging.md](D-messaging.md), [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C95 -> F-A155: **Timeline/composer/attachments**: draft-to-created-workspace handoff, fork history attachment, file/image review context, provisional title seed, pending initial create overlay and attention/read state. [C-timeline.md](C-timeline.md), [D-messaging.md](D-messaging.md), [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C95 -> F-B25: **Timeline/composer/attachments**: draft-to-created-workspace handoff, fork history attachment, file/image review context, provisional title seed, pending initial create overlay and attention/read state. [C-timeline.md](C-timeline.md), [D-messaging.md](D-messaging.md), [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C95 -> F-E45: **Timeline/composer/attachments**: draft-to-created-workspace handoff, fork history attachment, file/image review context, provisional title seed, pending initial create overlay and attention/read state. [C-timeline.md](C-timeline.md), [D-messaging.md](D-messaging.md), [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C95 -> F-F7: **Timeline/composer/attachments**: draft-to-created-workspace handoff, fork history attachment, file/image review context, provisional title seed, pending initial create overlay and attention/read state. [C-timeline.md](C-timeline.md), [D-messaging.md](D-messaging.md), [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C95 -> F-J36: **Timeline/composer/attachments**: draft-to-created-workspace handoff, fork history attachment, file/image review context, provisional title seed, pending initial create overlay and attention/read state. [C-timeline.md](C-timeline.md), [D-messaging.md](D-messaging.md), [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C95 -> F-K21: **Timeline/composer/attachments**: draft-to-created-workspace handoff, fork history attachment, file/image review context, provisional title seed, pending initial create overlay and attention/read state. [C-timeline.md](C-timeline.md), [D-messaging.md](D-messaging.md), [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C30 -> F-A7: **Git**: checkout source/base ref, worktree branch/cwd/backing ownership, exact subproject mapping, risky archive confirmation, dirty/ahead snapshots, metadata auto-name, Changes/Diff panel destinations. Git internals and operations are only in [p2-git.md](p2-git.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C30 -> F-D7: **Git**: checkout source/base ref, worktree branch/cwd/backing ownership, exact subproject mapping, risky archive confirmation, dirty/ahead snapshots, metadata auto-name, Changes/Diff panel destinations. Git internals and operations are only in [p2-git.md](p2-git.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C11 -> F-A106: **Forge/PR**: repository discovery, branch versus change-request worktree input, cross-repo trust block, PR URL open→merged latch, PR tab auto-open, forge links, merged URL consumed marker across restore. Provider-specific search/check/review/merge behavior is only in [p3-forge-review.md](p3-forge-review.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C11 -> F-D17: **Forge/PR**: repository discovery, branch versus change-request worktree input, cross-repo trust block, PR URL open→merged latch, PR tab auto-open, forge links, merged URL consumed marker across restore. Provider-specific search/check/review/merge behavior is only in [p3-forge-review.md](p3-forge-review.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C11 -> F-E1: **Forge/PR**: repository discovery, branch versus change-request worktree input, cross-repo trust block, PR URL open→merged latch, PR tab auto-open, forge links, merged URL consumed marker across restore. Provider-specific search/check/review/merge behavior is only in [p3-forge-review.md](p3-forge-review.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C93 -> F-D8: **Files/editor/transfer**: directory suggestions and selection, imported icon bytes, workspace file targets/line ranges, preview dirty guards, Open with…, desktop editor bridge and archived backing removal. Editing/transfer contracts are only in [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C93 -> F-F3: **Files/editor/transfer**: directory suggestions and selection, imported icon bytes, workspace file targets/line ranges, preview dirty guards, Open with…, desktop editor bridge and archived backing removal. Editing/transfer contracts are only in [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C93 -> F-H49: **Files/editor/transfer**: directory suggestions and selection, imported icon bytes, workspace file targets/line ranges, preview dirty guards, Open with…, desktop editor bridge and archived backing removal. Editing/transfer contracts are only in [p4-files.md](p4-files.md). [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C32 -> F-A9: **Hosts/daemon/settings**: all entity references pair host id with project/workspace id; online/capability gates, background discovery, owned subscriptions and generation/seq resume, daemon autoArchiveAfterMerge, terminal profiles/service-port policy. Device-local preferences include last selection, launch target, layout, collapse/order, keyboard overrides and service route choice. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C32 -> F-A79: **Hosts/daemon/settings**: all entity references pair host id with project/workspace id; online/capability gates, background discovery, owned subscriptions and generation/seq resume, daemon autoArchiveAfterMerge, terminal profiles/service-port policy. Device-local preferences include last selection, launch target, layout, collapse/order, keyboard overrides and service route choice. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C32 -> F-G29: **Hosts/daemon/settings**: all entity references pair host id with project/workspace id; online/capability gates, background discovery, owned subscriptions and generation/seq resume, daemon autoArchiveAfterMerge, terminal profiles/service-port policy. Device-local preferences include last selection, launch target, layout, collapse/order, keyboard overrides and service route choice. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C32 -> F-H36: **Hosts/daemon/settings**: all entity references pair host id with project/workspace id; online/capability gates, background discovery, owned subscriptions and generation/seq resume, daemon autoArchiveAfterMerge, terminal profiles/service-port policy. Device-local preferences include last selection, launch target, layout, collapse/order, keyboard overrides and service route choice. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C32 -> F-K38: **Hosts/daemon/settings**: all entity references pair host id with project/workspace id; online/capability gates, background discovery, owned subscriptions and generation/seq resume, daemon autoArchiveAfterMerge, terminal profiles/service-port policy. Device-local preferences include last selection, launch target, layout, collapse/order, keyboard overrides and service route choice. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C32 -> F-L37: **Hosts/daemon/settings**: all entity references pair host id with project/workspace id; online/capability gates, background discovery, owned subscriptions and generation/seq resume, daemon autoArchiveAfterMerge, terminal profiles/service-port policy. Device-local preferences include last selection, launch target, layout, collapse/order, keyboard overrides and service route choice. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C73 -> F-A7: **Terminals/services**: initial terminal launch, worktree auto-terminals/setup env, named script supervision, ports/URLs/health, activity contribution, stop/restart and archive kill escalation. Same cwd is not same owner; workspace id scopes each resource. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C73 -> F-G16: **Terminals/services**: initial terminal launch, worktree auto-terminals/setup env, named script supervision, ports/URLs/health, activity contribution, stop/restart and archive kill escalation. Same cwd is not same owner; workspace id scopes each resource. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C73 -> F-M10: **Terminals/services**: initial terminal launch, worktree auto-terminals/setup env, named script supervision, ports/URLs/health, activity contribution, stop/restart and archive kill escalation. Same cwd is not same owner; workspace id scopes each resource. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C66 -> F-A3: **Permissions/questions/attention**: needs_input status derives agent permissions/questions; Mark read does not clear permission attention. Cross-repo untrustedSource blocks automatic setup/scripts; explicit setup release changes that persisted gate. No workspace label or pin operation resolves an agent question. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C66 -> F-A261: **Permissions/questions/attention**: needs_input status derives agent permissions/questions; Mark read does not clear permission attention. Cross-repo untrustedSource blocks automatic setup/scripts; explicit setup release changes that persisted gate. No workspace label or pin operation resolves an agent question. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C66 -> F-G31: **Permissions/questions/attention**: needs_input status derives agent permissions/questions; Mark read does not clear permission attention. Cross-repo untrustedSource blocks automatic setup/scripts; explicit setup release changes that persisted gate. No workspace label or pin operation resolves an agent question. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C66 -> F-J36: **Permissions/questions/attention**: needs_input status derives agent permissions/questions; Mark read does not clear permission attention. Cross-repo untrustedSource blocks automatic setup/scripts; explicit setup release changes that persisted gate. No workspace label or pin operation resolves an agent question. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C8 -> F-A3: **Plugins/metadata providers**: workspace.create.before and worktree create hooks, metadata-generated title/branch, plugin workspace/agent panels and allowed pane locations. Hook failure can roll back a newly created worktree; metadata failure does not roll back workspace creation. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C8 -> F-A100: **Plugins/metadata providers**: workspace.create.before and worktree create hooks, metadata-generated title/branch, plugin workspace/agent panels and allowed pane locations. Hook failure can roll back a newly created worktree; metadata failure does not roll back workspace creation. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C8 -> F-D17: **Plugins/metadata providers**: workspace.create.before and worktree create hooks, metadata-generated title/branch, plugin workspace/agent panels and allowed pane locations. Hook failure can roll back a newly created worktree; metadata failure does not roll back workspace creation. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C8 -> F-M21: **Plugins/metadata providers**: workspace.create.before and worktree create hooks, metadata-generated title/branch, plugin workspace/agent panels and allowed pane locations. Hook failure can roll back a newly created worktree; metadata failure does not roll back workspace creation. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C93 -> F-A6: **Navigation/command center/desktop**: active workspace identity, route recovery, archive redirect, numbered sidebar targets, tab focus/side-sheet placement, external editor/window bridge. Local tab arrangement is not broadcast as daemon workspace state. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C93 -> F-F14: **Navigation/command center/desktop**: active workspace identity, route recovery, archive redirect, numbered sidebar targets, tab focus/side-sheet placement, external editor/window bridge. Local tab arrangement is not broadcast as daemon workspace state. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C93 -> F-H45: **Navigation/command center/desktop**: active workspace identity, route recovery, archive redirect, numbered sidebar targets, tab focus/side-sheet placement, external editor/window bridge. Local tab arrangement is not broadcast as daemon workspace state. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-C93 -> F-K40: **Navigation/command center/desktop**: active workspace identity, route recovery, archive redirect, numbered sidebar targets, tab focus/side-sheet placement, external editor/window bridge. Local tab arrangement is not broadcast as daemon workspace state. [Survey: parts/p1-projects-workspaces.md#cross-area-connections]

F-D12 -> F-A193: Forge / pull request (other surveyor): CheckoutSession also serves checkout_pr_create/merge/status, forge auto-merge, check details, PR timeline, forge search; `checkout_status_update.prStatus`; A/git/{forge*,pr-*,pull-request-*,check-presentation*,use-pr-status-query,use-forge-search-query,pull-request-panel/*,forges/*}; `buildGitActions` PR/merge/auto-merge actions; `merge-capability.ts`; `changes-open-pull-request`; forge setup callout (`authState`). [Survey: parts/p2-git.md#cross-area-connections]

F-D12 -> F-E20: Forge / pull request (other surveyor): CheckoutSession also serves checkout_pr_create/merge/status, forge auto-merge, check details, PR timeline, forge search; `checkout_status_update.prStatus`; A/git/{forge*,pr-*,pull-request-*,check-presentation*,use-pr-status-query,use-forge-search-query,pull-request-panel/*,forges/*}; `buildGitActions` PR/merge/auto-merge actions; `merge-capability.ts`; `changes-open-pull-request`; forge setup callout (`authState`). [Survey: parts/p2-git.md#cross-area-connections]

F-D12 -> F-A14: Session events subscription (`session.events.set_subscription.request`, `observeEvents`) carries `checkout_status_update` together with attention/permission/provider-subagent events (session-context.tsx:481-525). [Survey: parts/p2-git.md#cross-area-connections]

F-D12 -> F-A106: Session events subscription (`session.events.set_subscription.request`, `observeEvents`) carries `checkout_status_update` together with attention/permission/provider-subagent events (session-context.tsx:481-525). [Survey: parts/p2-git.md#cross-area-connections]

F-D12 -> F-A260: Session events subscription (`session.events.set_subscription.request`, `observeEvents`) carries `checkout_status_update` together with attention/permission/provider-subagent events (session-context.tsx:481-525). [Survey: parts/p2-git.md#cross-area-connections]

F-D12 -> F-J36: Session events subscription (`session.events.set_subscription.request`, `observeEvents`) carries `checkout_status_update` together with attention/permission/provider-subagent events (session-context.tsx:481-525). [Survey: parts/p2-git.md#cross-area-connections]

F-D17 -> F-C58: Workspace registry/sidebar: `workspace_update` (gitRuntime, diffStat), `WorkspaceGitObserverService.onBranchChanged` (workspace auto-title/branch sync), `emitWorkspaceUpdateForCwd`, `getCheckout` (project lite). [Survey: parts/p2-git.md#cross-area-connections]

F-D17 -> F-K10: Workspace registry/sidebar: `workspace_update` (gitRuntime, diffStat), `WorkspaceGitObserverService.onBranchChanged` (workspace auto-title/branch sync), `emitWorkspaceUpdateForCwd`, `getCheckout` (project lite). [Survey: parts/p2-git.md#cross-area-connections]

F-D1 -> F-A6: Worktrees: `isPaseoOwnedWorktree`, `mainRepoRoot`, base ref from worktree metadata `paseo/worktree.json`, archive flow (`paseo_worktree_archive_request`, `useWorkspaceArchive`, archive warning), `createBranchFromBase`, `create-worktree` mutation reason, `rebindPaseoWorktreeChangeRequestHint` on rename. [Survey: parts/p2-git.md#cross-area-connections]

F-D1 -> F-C84: Worktrees: `isPaseoOwnedWorktree`, `mainRepoRoot`, base ref from worktree metadata `paseo/worktree.json`, archive flow (`paseo_worktree_archive_request`, `useWorkspaceArchive`, archive warning), `createBranchFromBase`, `create-worktree` mutation reason, `rebindPaseoWorktreeChangeRequestHint` on rename. [Survey: parts/p2-git.md#cross-area-connections]

F-D20 -> F-B25: Review / composer attachments: `useWorkingDiff` -> A/review (inline comments, review feedback, `ReviewFeedbackAction`) and `useWorkspaceAttachmentsStore` (review attachment published for the composer); "Add to chat" file action (`useAddFileToChat`). [Survey: parts/p2-git.md#cross-area-connections]

F-D20 -> F-C68: Review / composer attachments: `useWorkingDiff` -> A/review (inline comments, review feedback, `ReviewFeedbackAction`) and `useWorkspaceAttachmentsStore` (review attachment published for the composer); "Add to chat" file action (`useAddFileToChat`). [Survey: parts/p2-git.md#cross-area-connections]

F-D20 -> F-E49: Review / composer attachments: `useWorkingDiff` -> A/review (inline comments, review feedback, `ReviewFeedbackAction`) and `useWorkspaceAttachmentsStore` (review attachment published for the composer); "Add to chat" file action (`useAddFileToChat`). [Survey: parts/p2-git.md#cross-area-connections]

F-D20 -> F-F7: Review / composer attachments: `useWorkingDiff` -> A/review (inline comments, review feedback, `ReviewFeedbackAction`) and `useWorkspaceAttachmentsStore` (review attachment published for the composer); "Add to chat" file action (`useAddFileToChat`). [Survey: parts/p2-git.md#cross-area-connections]

F-D15 -> F-C86: Workspace tabs/layout: `working_diff`, `changes_tree`, `commit_diff` tab targets, `usePanelState` persistence, `openPreferredTarget` to open files in the file editor panel, `FOCUSED_PANE_PLACEMENT`. [Survey: parts/p2-git.md#cross-area-connections]

F-D15 -> F-F14: Workspace tabs/layout: `working_diff`, `changes_tree`, `commit_diff` tab targets, `usePanelState` persistence, `openPreferredTarget` to open files in the file editor panel, `FOCUSED_PANE_PLACEMENT`. [Survey: parts/p2-git.md#cross-area-connections]

F-D1 -> F-F6: File features: download (`useFileDownload`), duplicate (`client.duplicateFileEntry`), reveal (desktop open targets), absolute-path building (`buildAbsoluteExplorerPath`), copy path. [Survey: parts/p2-git.md#cross-area-connections]

F-D1 -> F-H49: File features: download (`useFileDownload`), duplicate (`client.duplicateFileEntry`), reveal (desktop open targets), absolute-path building (`buildAbsoluteExplorerPath`), copy path. [Survey: parts/p2-git.md#cross-area-connections]

F-D12 -> F-A3: Agent-commands cache: branch change in status update resets draft agent commands (`resetDraftAgentCommandsForCheckout`). [Survey: parts/p2-git.md#cross-area-connections]

F-D12 -> F-B7: Agent-commands cache: branch change in status update resets draft agent commands (`resetDraftAgentCommandsForCheckout`). [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-A3: Agent / LLM: commit message generation spawns an internal non-persisted agent (`createAgentStructuredTextGeneration`, AgentManager, ProviderSnapshotManager, `resolveStructuredGenerationProviders`, daemon config style keys `commitMessage` / `pullRequest` via `buildMetadataPrompt`). [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-A79: Agent / LLM: commit message generation spawns an internal non-persisted agent (`createAgentStructuredTextGeneration`, AgentManager, ProviderSnapshotManager, `resolveStructuredGenerationProviders`, daemon config style keys `commitMessage` / `pullRequest` via `buildMetadataPrompt`). [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-H59: Agent / LLM: commit message generation spawns an internal non-persisted agent (`createAgentStructuredTextGeneration`, AgentManager, ProviderSnapshotManager, `resolveStructuredGenerationProviders`, daemon config style keys `commitMessage` / `pullRequest` via `buildMetadataPrompt`). [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-L19: Agent / LLM: commit message generation spawns an internal non-persisted agent (`createAgentStructuredTextGeneration`, AgentManager, ProviderSnapshotManager, `resolveStructuredGenerationProviders`, daemon config style keys `commitMessage` / `pullRequest` via `buildMetadataPrompt`). [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-A2: Server infrastructure: `WorkspaceRuntimeStore` (diff manager), `file-observer` (watchers), `session/owned-subscriptions` (`SessionDelivery.begin("diffs", ...)` owner lifecycle), `authorization/operation-permissions.ts`, relay frame limit (`MAX_RELAY_WEBSOCKET_MESSAGE_BYTES`) sizing `CHECKOUT_DIFF_MAX_STRUCTURED_BYTES`, `diff-highlighter.ts` (`parseAndHighlightDiff`), `run-git-command.ts` / `git-process-scheduler.ts`. [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-A260: Server infrastructure: `WorkspaceRuntimeStore` (diff manager), `file-observer` (watchers), `session/owned-subscriptions` (`SessionDelivery.begin("diffs", ...)` owner lifecycle), `authorization/operation-permissions.ts`, relay frame limit (`MAX_RELAY_WEBSOCKET_MESSAGE_BYTES`) sizing `CHECKOUT_DIFF_MAX_STRUCTURED_BYTES`, `diff-highlighter.ts` (`parseAndHighlightDiff`), `run-git-command.ts` / `git-process-scheduler.ts`. [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-C68: Server infrastructure: `WorkspaceRuntimeStore` (diff manager), `file-observer` (watchers), `session/owned-subscriptions` (`SessionDelivery.begin("diffs", ...)` owner lifecycle), `authorization/operation-permissions.ts`, relay frame limit (`MAX_RELAY_WEBSOCKET_MESSAGE_BYTES`) sizing `CHECKOUT_DIFF_MAX_STRUCTURED_BYTES`, `diff-highlighter.ts` (`parseAndHighlightDiff`), `run-git-command.ts` / `git-process-scheduler.ts`. [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-F15: Server infrastructure: `WorkspaceRuntimeStore` (diff manager), `file-observer` (watchers), `session/owned-subscriptions` (`SessionDelivery.begin("diffs", ...)` owner lifecycle), `authorization/operation-permissions.ts`, relay frame limit (`MAX_RELAY_WEBSOCKET_MESSAGE_BYTES`) sizing `CHECKOUT_DIFF_MAX_STRUCTURED_BYTES`, `diff-highlighter.ts` (`parseAndHighlightDiff`), `run-git-command.ts` / `git-process-scheduler.ts`. [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-H19: Server infrastructure: `WorkspaceRuntimeStore` (diff manager), `file-observer` (watchers), `session/owned-subscriptions` (`SessionDelivery.begin("diffs", ...)` owner lifecycle), `authorization/operation-permissions.ts`, relay frame limit (`MAX_RELAY_WEBSOCKET_MESSAGE_BYTES`) sizing `CHECKOUT_DIFF_MAX_STRUCTURED_BYTES`, `diff-highlighter.ts` (`parseAndHighlightDiff`), `run-git-command.ts` / `git-process-scheduler.ts`. [Survey: parts/p2-git.md#cross-area-connections]

F-D22 -> F-I2: Server infrastructure: `WorkspaceRuntimeStore` (diff manager), `file-observer` (watchers), `session/owned-subscriptions` (`SessionDelivery.begin("diffs", ...)` owner lifecycle), `authorization/operation-permissions.ts`, relay frame limit (`MAX_RELAY_WEBSOCKET_MESSAGE_BYTES`) sizing `CHECKOUT_DIFF_MAX_STRUCTURED_BYTES`, `diff-highlighter.ts` (`parseAndHighlightDiff`), `run-git-command.ts` / `git-process-scheduler.ts`. [Survey: parts/p2-git.md#cross-area-connections]

F-D8 -> F-A6: Settings: app settings `codeFontSize`, `monoFontFamily`, `pullRequestOpenLocation`; global changes preferences; shortcuts (`archive-workspace`, `workspace-tab-target-changes`). [Survey: parts/p2-git.md#cross-area-connections]

F-D8 -> F-C90: Settings: app settings `codeFontSize`, `monoFontFamily`, `pullRequestOpenLocation`; global changes preferences; shortcuts (`archive-workspace`, `workspace-tab-target-changes`). [Survey: parts/p2-git.md#cross-area-connections]

F-D8 -> F-K37: Settings: app settings `codeFontSize`, `monoFontFamily`, `pullRequestOpenLocation`; global changes preferences; shortcuts (`archive-workspace`, `workspace-tab-target-changes`). [Survey: parts/p2-git.md#cross-area-connections]

F-D8 -> F-L1: Settings: app settings `codeFontSize`, `monoFontFamily`, `pullRequestOpenLocation`; global changes preferences; shortcuts (`archive-workspace`, `workspace-tab-target-changes`). [Survey: parts/p2-git.md#cross-area-connections]

F-D13 -> F-H34: Connection/reconnect: `invalidateServerDataQueriesAfterReconnect` / `invalidateCheckoutGitQueriesForServer`, `useReplicaQuery` + push router re-subscription of diff routes, host capability flags in `status.server_info.features`. [Survey: parts/p2-git.md#cross-area-connections]

F-D13 -> F-K29: Connection/reconnect: `invalidateServerDataQueriesAfterReconnect` / `invalidateCheckoutGitQueriesForServer`, `useReplicaQuery` + push router re-subscription of diff routes, host capability flags in `status.server_info.features`. [Survey: parts/p2-git.md#cross-area-connections]

F-E1 -> F-C68: Workspace git service / snapshot / `checkout_status_update` (git surveyor): PR status poll, `forge` resolution, `githubRuntime` in workspace descriptors; `notifyGitMutation(..., {invalidateForge:true})`. / Sidebar / workspace list: `PrBadge`, check indicator, hover card, context menu (`prHint` from `githubRuntime.pullRequest` + `forge`). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E1 -> F-D12: Workspace git service / snapshot / `checkout_status_update` (git surveyor): PR status poll, `forge` resolution, `githubRuntime` in workspace descriptors; `notifyGitMutation(..., {invalidateForge:true})`. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E1 -> F-G29: Workspace git service / snapshot / `checkout_status_update` (git surveyor): PR status poll, `forge` resolution, `githubRuntime` in workspace descriptors; `notifyGitMutation(..., {invalidateForge:true})`. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E37 -> F-C84: Workspace creation & worktrees (workspaces surveyor): `checkoutSource`, `resolveWorktreeCreationIntent`, worktree metadata `changeRequestLookupTarget`, `create_paseo_worktree_request`, `workspace.create.request`, plugin hook `worktree.create.before`, MCP `workspace create` tool. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E37 -> F-D7: Workspace creation & worktrees (workspaces surveyor): `checkoutSource`, `resolveWorktreeCreationIntent`, worktree metadata `changeRequestLookupTarget`, `create_paseo_worktree_request`, `workspace.create.request`, plugin hook `worktree.create.before`, MCP `workspace create` tool. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E37 -> F-I12: Workspace creation & worktrees (workspaces surveyor): `checkoutSource`, `resolveWorktreeCreationIntent`, worktree metadata `changeRequestLookupTarget`, `create_paseo_worktree_request`, `workspace.create.request`, plugin hook `worktree.create.before`, MCP `workspace create` tool. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E37 -> F-M20: Workspace creation & worktrees (workspaces surveyor): `checkoutSource`, `resolveWorktreeCreationIntent`, worktree metadata `changeRequestLookupTarget`, `create_paseo_worktree_request`, `workspace.create.request`, plugin hook `worktree.create.before`, MCP `workspace create` tool. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E29 -> F-B25: Composer / attachments (composer surveyor): forge attachments (`forge_change_request`, `forge_issue`, legacy `github_*`), `review` attachment, PR context text attachments, auto-attach on pasted URLs, workspace attachment store, send behavior setting. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E29 -> F-C68: Composer / attachments (composer surveyor): forge attachments (`forge_change_request`, `forge_issue`, legacy `github_*`), `review` attachment, PR context text attachments, auto-attach on pasted URLs, workspace attachment store, send behavior setting. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E29 -> F-L2: Composer / attachments (composer surveyor): forge attachments (`forge_change_request`, `forge_issue`, legacy `github_*`), `review` attachment, PR context text attachments, auto-attach on pasted URLs, workspace attachment store, send behavior setting. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E36 -> F-A3: Agent prompt building: `prompt-attachments.ts` (forge + review rendering), branch-name seed from attachments. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E36 -> F-B23: Agent prompt building: `prompt-attachments.ts` (forge + review rendering), branch-name seed from attachments. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E36 -> F-D17: Agent prompt building: `prompt-attachments.ts` (forge + review rendering), branch-name seed from attachments. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E1 -> F-K10: Sidebar / workspace list: `PrBadge`, check indicator, hover card, context menu (`prHint` from `githubRuntime.pullRequest` + `forge`). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E25 -> F-C86: Workspace tabs/layout store: `pull_request` tab kind, auto-open, `pullRequestOpenLocation` setting, command center + keyboard panel target, launcher entry. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E25 -> F-K40: Workspace tabs/layout store: `pull_request` tab kind, auto-open, `pullRequestOpenLocation` setting, command center + keyboard panel target, launcher entry. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E25 -> F-L10: Workspace tabs/layout store: `pull_request` tab kind, auto-open, `pullRequestOpenLocation` setting, command center + keyboard panel target, launcher entry. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E49 -> F-A98: Diff/changes pane: review gutter + feedback action live in `APP/git/diff-pane.tsx`; diff query/modes (`uncommitted|base|all`). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E49 -> F-C86: Diff/changes pane: review gutter + feedback action live in `APP/git/diff-pane.tsx`; diff query/modes (`uncommitted|base|all`). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E49 -> F-D22: Diff/changes pane: review gutter + feedback action live in `APP/git/diff-pane.tsx`; diff query/modes (`uncommitted|base|all`). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E41 -> F-C83: Add-project flow: GitHub repo search/clone RPCs and CLI `paseo clone`. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E27 -> F-B26: Settings: `pullRequestOpenLocation`, `sendBehavior`, daemon config `autoArchiveAfterMerge`, metadata prompt style key `pullRequest` (git metadata generator). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E27 -> F-D1: Settings: `pullRequestOpenLocation`, `sendBehavior`, daemon config `autoArchiveAfterMerge`, metadata prompt style key `pullRequest` (git metadata generator). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E27 -> F-H59: Settings: `pullRequestOpenLocation`, `sendBehavior`, daemon config `autoArchiveAfterMerge`, metadata prompt style key `pullRequest` (git metadata generator). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E27 -> F-L19: Settings: `pullRequestOpenLocation`, `sendBehavior`, daemon config `autoArchiveAfterMerge`, metadata prompt style key `pullRequest` (git metadata generator). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E1 -> F-A106: Server info features gating (`serverInfo.features`): `forgeSearch`, `forgeCheckDetails`, `githubCheckDetails`, `forgeProviders`, `checkoutForgeSetAutoMerge`, `checkoutGithubSetAutoMerge`, `checkoutRefresh`, `projectGithubClone`, `workspaceGithubRepositorySearch`. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E1 -> F-C58: Server info features gating (`serverInfo.features`): `forgeSearch`, `forgeCheckDetails`, `githubCheckDetails`, `forgeProviders`, `checkoutForgeSetAutoMerge`, `checkoutGithubSetAutoMerge`, `checkoutRefresh`, `projectGithubClone`, `workspaceGithubRepositorySearch`. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E1 -> F-D7: Server info features gating (`serverInfo.features`): `forgeSearch`, `forgeCheckDetails`, `githubCheckDetails`, `forgeProviders`, `checkoutForgeSetAutoMerge`, `checkoutGithubSetAutoMerge`, `checkoutRefresh`, `projectGithubClone`, `workspaceGithubRepositorySearch`. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E1 -> F-A260: Authorization: operation permission table for all RPCs above. [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E54 -> F-A7: Auto-archive-on-merge: consumes forge snapshot (documented elsewhere). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E50 -> F-A3: Agent-structured generation providers: PR title/body generation (`createAgentStructuredTextGeneration`). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-E50 -> F-A100: Agent-structured generation providers: PR title/body generation (`createAgentStructuredTextGeneration`). [Survey: parts/p3-forge-review.md#cross-area-connections]

F-F7 -> F-B25: Composer / attachments: `file.upload.request` + `UploadedFileAttachment` (`uploadFileAttachments`, `composer/actions.ts:126`, `FILE_UPLOAD_MAX_BYTES = 50 MiB` in `attachments/upload-limit.ts`) and `AgentAttachment` union in `create_agent`/send message; "Add to chat" context action (`panels/use-add-file-to-chat.ts`), workspace file drag source (`attachments/use-workspace-file-drag-source*`), file drop zones (`components/file-drop/*`, `composer/attachments/drop.ts`, `terminal/drop/terminal-file-drop.ts`). [Survey: parts/p4-files.md#cross-area-connections]

F-F7 -> F-C31: Composer / attachments: `file.upload.request` + `UploadedFileAttachment` (`uploadFileAttachments`, `composer/actions.ts:126`, `FILE_UPLOAD_MAX_BYTES = 50 MiB` in `attachments/upload-limit.ts`) and `AgentAttachment` union in `create_agent`/send message; "Add to chat" context action (`panels/use-add-file-to-chat.ts`), workspace file drag source (`attachments/use-workspace-file-drag-source*`), file drop zones (`components/file-drop/*`, `composer/attachments/drop.ts`, `terminal/drop/terminal-file-drop.ts`). [Survey: parts/p4-files.md#cross-area-connections]

F-F7 -> F-G10: Composer / attachments: `file.upload.request` + `UploadedFileAttachment` (`uploadFileAttachments`, `composer/actions.ts:126`, `FILE_UPLOAD_MAX_BYTES = 50 MiB` in `attachments/upload-limit.ts`) and `AgentAttachment` union in `create_agent`/send message; "Add to chat" context action (`panels/use-add-file-to-chat.ts`), workspace file drag source (`attachments/use-workspace-file-drag-source*`), file drop zones (`components/file-drop/*`, `composer/attachments/drop.ts`, `terminal/drop/terminal-file-drop.ts`). [Survey: parts/p4-files.md#cross-area-connections]

F-F9 -> F-A133: Chat/timeline: assistant file links (`assistant-file-links/*`) open file tabs with line ranges and use `listDirectory`/`directory_suggestions`; Markdown preview reuses `MarkdownRenderer`. [Survey: parts/p4-files.md#cross-area-connections]

F-F9 -> F-C6: Chat/timeline: assistant file links (`assistant-file-links/*`) open file tabs with line ranges and use `listDirectory`/`directory_suggestions`; Markdown preview reuses `MarkdownRenderer`. / Workspace/tabs: file tab target kind, tab modified indicator and discard confirmation (`screens/workspace/*`), open-location settings (`openInSidePane`), "Open with..." tab menu, `workspace-file-open-command` (`screens/workspace/workspace-file-open-command.ts`). [Survey: parts/p4-files.md#cross-area-connections]

F-F9 -> F-L10: Workspace/tabs: file tab target kind, tab modified indicator and discard confirmation (`screens/workspace/*`), open-location settings (`openInSidePane`), "Open with..." tab menu, `workspace-file-open-command` (`screens/workspace/workspace-file-open-command.ts`). [Survey: parts/p4-files.md#cross-area-connections]

F-F15 -> F-C90: Git: diff pane shares `FileActionsContextMenuContent` and `useFileDownload`; `server/file-observer` powers working-tree watch for `WorkspaceGitService`; rename uses `git mv`; forge open-in-browser links (`git/forge`). [Survey: parts/p4-files.md#cross-area-connections]

F-F15 -> F-D22: Git: diff pane shares `FileActionsContextMenuContent` and `useFileDownload`; `server/file-observer` powers working-tree watch for `WorkspaceGitService`; rename uses `git mv`; forge open-in-browser links (`git/forge`). [Survey: parts/p4-files.md#cross-area-connections]

F-F15 -> F-E1: Git: diff pane shares `FileActionsContextMenuContent` and `useFileDownload`; `server/file-observer` powers working-tree watch for `WorkspaceGitService`; rename uses `git mv`; forge open-in-browser links (`git/forge`). [Survey: parts/p4-files.md#cross-area-connections]

F-F15 -> F-G36: Git: diff pane shares `FileActionsContextMenuContent` and `useFileDownload`; `server/file-observer` powers working-tree watch for `WorkspaceGitService`; rename uses `git mv`; forge open-in-browser links (`git/forge`). [Survey: parts/p4-files.md#cross-area-connections]

F-F3 -> F-A10: Connection layer: `connection types` (directTcp vs relay/SSH/socket) decide HTTP download vs session socket; binary-frame demux (`protocol/binary-frames`), `ownership` / `SessionDelivery` (`session/owned-subscriptions`) for file subscriptions and uploads, WebSocket message size limits, relay 32 MiB frame cap. [Survey: parts/p4-files.md#cross-area-connections]

F-F3 -> F-H19: Connection layer: `connection types` (directTcp vs relay/SSH/socket) decide HTTP download vs session socket; binary-frame demux (`protocol/binary-frames`), `ownership` / `SessionDelivery` (`session/owned-subscriptions`) for file subscriptions and uploads, WebSocket message size limits, relay 32 MiB frame cap. [Survey: parts/p4-files.md#cross-area-connections]

F-F1 -> F-H29: Plugins: `addFilePanel`, `PluginFile`, `DocumentFrame`, `useFileText/Bytes/Url`; plugin install/host (`plugins/*`). [Survey: parts/p4-files.md#cross-area-connections]

F-F1 -> F-M21: Plugins: `addFilePanel`, `PluginFile`, `DocumentFrame`, `useFileText/Bytes/Url`; plugin install/host (`plugins/*`). [Survey: parts/p4-files.md#cross-area-connections]

F-F4 -> F-L8: Settings: `filePanelAssociations`, `vimKeybindings`, `openInSidePane`; preferred editor in AsyncStorage; `usePanelStore` explorer keys. [Survey: parts/p4-files.md#cross-area-connections]

F-F14 -> F-H45: Desktop shell: `packages/desktop` editor-targets IPC, `getIsElectron`, local daemon id. [Survey: parts/p4-files.md#cross-area-connections]

F-F16 -> F-A3: Directory sync: agent/workspace/project lists (runtime slice). [Survey: parts/p4-files.md#cross-area-connections]

F-F16 -> F-C98: Directory sync: agent/workspace/project lists (runtime slice). [Survey: parts/p4-files.md#cross-area-connections]

F-F16 -> F-H62: Directory sync: agent/workspace/project lists (runtime slice). [Survey: parts/p4-files.md#cross-area-connections]

F-F16 -> F-C13: Daemon features: `server_info.features` flags above; `projects/workspaces` registries (workspace root = file scope). [Survey: parts/p4-files.md#cross-area-connections]

F-F16 -> F-H59: Daemon features: `server_info.features` flags above; `projects/workspaces` registries (workspace root = file scope). [Survey: parts/p4-files.md#cross-area-connections]

F-F11 -> F-C68: i18n: keys under `workspace.fileExplorer.*`, `workspace.fileActions.*`, `panels.file.*`, `panels.file.editor.*`, `panels.file.filePanels.*`, `downloads.*`, `paneFind.*`. [Survey: parts/p4-files.md#cross-area-connections]

F-F11 -> F-L8: i18n: keys under `workspace.fileExplorer.*`, `workspace.fileActions.*`, `panels.file.*`, `panels.file.editor.*`, `panels.file.filePanels.*`, `downloads.*`, `paneFind.*`. [Survey: parts/p4-files.md#cross-area-connections]

F-G14 -> F-A3: Agents/sessions: agent tools (`create_terminal`, `capture_terminal`, `send_terminal_keys`, `list_workspace_scripts`, `start/stop_workspace_script`, `browser_*`) registered in `paseo-tools.ts`; agent workspace decides terminal workspace; `agentId` terminals rejected on create; hooks for Claude/Codex/OpenCode feed terminal activity; browser tool calls carry `agentId/cwd/workspaceId`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G14 -> F-A160: Agents/sessions: agent tools (`create_terminal`, `capture_terminal`, `send_terminal_keys`, `list_workspace_scripts`, `start/stop_workspace_script`, `browser_*`) registered in `paseo-tools.ts`; agent workspace decides terminal workspace; `agentId` terminals rejected on create; hooks for Claude/Codex/OpenCode feed terminal activity; browser tool calls carry `agentId/cwd/workspaceId`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G14 -> F-C31: Agents/sessions: agent tools (`create_terminal`, `capture_terminal`, `send_terminal_keys`, `list_workspace_scripts`, `start/stop_workspace_script`, `browser_*`) registered in `paseo-tools.ts`; agent workspace decides terminal workspace; `agentId` terminals rejected on create; hooks for Claude/Codex/OpenCode feed terminal activity; browser tool calls carry `agentId/cwd/workspaceId`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G14 -> F-I18: Agents/sessions: agent tools (`create_terminal`, `capture_terminal`, `send_terminal_keys`, `list_workspace_scripts`, `start/stop_workspace_script`, `browser_*`) registered in `paseo-tools.ts`; agent workspace decides terminal workspace; `agentId` terminals rejected on create; hooks for Claude/Codex/OpenCode feed terminal activity; browser tool calls carry `agentId/cwd/workspaceId`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G31 -> F-A6: Workspaces/projects: terminals and scripts keyed by `workspaceId`; archive teardown (`killTerminalsForWorkspace`, runtime store removal, port plan release); worktree creation runs `worktree.setup` then `worktree.terminals`; branch rename re-routes service hostnames; workspace descriptor carries `scripts[]` and terminal activity roll-up status; project settings editor writes `paseo.json` scripts; daemon `worktrees.servicePorts`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G31 -> F-C82: Workspaces/projects: terminals and scripts keyed by `workspaceId`; archive teardown (`killTerminalsForWorkspace`, runtime store removal, port plan release); worktree creation runs `worktree.setup` then `worktree.terminals`; branch rename re-routes service hostnames; workspace descriptor carries `scripts[]` and terminal activity roll-up status; project settings editor writes `paseo.json` scripts; daemon `worktrees.servicePorts`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G31 -> F-D17: Workspaces/projects: terminals and scripts keyed by `workspaceId`; archive teardown (`killTerminalsForWorkspace`, runtime store removal, port plan release); worktree creation runs `worktree.setup` then `worktree.terminals`; branch rename re-routes service hostnames; workspace descriptor carries `scripts[]` and terminal activity roll-up status; project settings editor writes `paseo.json` scripts; daemon `worktrees.servicePorts`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G31 -> F-F11: Workspaces/projects: terminals and scripts keyed by `workspaceId`; archive teardown (`killTerminalsForWorkspace`, runtime store removal, port plan release); worktree creation runs `worktree.setup` then `worktree.terminals`; branch rename re-routes service hostnames; workspace descriptor carries `scripts[]` and terminal activity roll-up status; project settings editor writes `paseo.json` scripts; daemon `worktrees.servicePorts`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G31 -> F-H59: Workspaces/projects: terminals and scripts keyed by `workspaceId`; archive teardown (`killTerminalsForWorkspace`, runtime store removal, port plan release); worktree creation runs `worktree.setup` then `worktree.terminals`; branch rename re-routes service hostnames; workspace descriptor carries `scripts[]` and terminal activity roll-up status; project settings editor writes `paseo.json` scripts; daemon `worktrees.servicePorts`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G31 -> F-K29: Workspaces/projects: terminals and scripts keyed by `workspaceId`; archive teardown (`killTerminalsForWorkspace`, runtime store removal, port plan release); worktree creation runs `worktree.setup` then `worktree.terminals`; branch rename re-routes service hostnames; workspace descriptor carries `scripts[]` and terminal activity roll-up status; project settings editor writes `paseo.json` scripts; daemon `worktrees.servicePorts`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G31 -> F-L7: Workspaces/projects: terminals and scripts keyed by `workspaceId`; archive teardown (`killTerminalsForWorkspace`, runtime store removal, port plan release); worktree creation runs `worktree.setup` then `worktree.terminals`; branch rename re-routes service hostnames; workspace descriptor carries `scripts[]` and terminal activity roll-up status; project settings editor writes `paseo.json` scripts; daemon `worktrees.servicePorts`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G29 -> F-A2: Host/daemon: daemon config keys `browserTools.enabled`, `enableTerminalAgentHooks`, `terminalProfiles`, `serviceProxy.{listen,publicBaseUrl}`, `worktrees.servicePorts`, env `PASEO_SERVICE_PROXY_*`; terminal worker lifecycle tied to daemon stop/restart (`keepTerminals`); loopback `/api/terminal-activity`; express mount order (service proxy before auth/host allowlist); WS upgrade sharing; server info `features` flags for terminal capabilities. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G29 -> F-C73: Host/daemon: daemon config keys `browserTools.enabled`, `enableTerminalAgentHooks`, `terminalProfiles`, `serviceProxy.{listen,publicBaseUrl}`, `worktrees.servicePorts`, env `PASEO_SERVICE_PROXY_*`; terminal worker lifecycle tied to daemon stop/restart (`keepTerminals`); loopback `/api/terminal-activity`; express mount order (service proxy before auth/host allowlist); WS upgrade sharing; server info `features` flags for terminal capabilities. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G29 -> F-H59: Host/daemon: daemon config keys `browserTools.enabled`, `enableTerminalAgentHooks`, `terminalProfiles`, `serviceProxy.{listen,publicBaseUrl}`, `worktrees.servicePorts`, env `PASEO_SERVICE_PROXY_*`; terminal worker lifecycle tied to daemon stop/restart (`keepTerminals`); loopback `/api/terminal-activity`; express mount order (service proxy before auth/host allowlist); WS upgrade sharing; server info `features` flags for terminal capabilities. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G29 -> F-L19: Host/daemon: daemon config keys `browserTools.enabled`, `enableTerminalAgentHooks`, `terminalProfiles`, `serviceProxy.{listen,publicBaseUrl}`, `worktrees.servicePorts`, env `PASEO_SERVICE_PROXY_*`; terminal worker lifecycle tied to daemon stop/restart (`keepTerminals`); loopback `/api/terminal-activity`; express mount order (service proxy before auth/host allowlist); WS upgrade sharing; server info `features` flags for terminal capabilities. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G6 -> F-I1: Notifications/attention: `terminal_attention_required` -> OS/push notifications, `computeNotificationPlan` with client presence heartbeat (`focusedTerminalId`), sidebar status buckets. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G6 -> F-J7: Notifications/attention: `terminal_attention_required` -> OS/push notifications, `computeNotificationPlan` with client presence heartbeat (`focusedTerminalId`), sidebar status buckets. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G6 -> F-K10: Notifications/attention: `terminal_attention_required` -> OS/push notifications, `computeNotificationPlan` with client presence heartbeat (`focusedTerminalId`), sidebar status buckets. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G37 -> F-B25: Attachments/composer: browser "annotate element" adds `browser_element` attachments to the composer; terminal file paste/drop uses `client.uploadFile` (file upload RPC, 50 MiB limit); terminal drop uses desktop host bridge (`webUtils.getPathForFile`). [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G37 -> F-F7: Attachments/composer: browser "annotate element" adds `browser_element` attachments to the composer; terminal file paste/drop uses `client.uploadFile` (file upload RPC, 50 MiB limit); terminal drop uses desktop host bridge (`webUtils.getPathForFile`). [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G37 -> F-H49: Attachments/composer: browser "annotate element" adds `browser_element` attachments to the composer; terminal file paste/drop uses `client.uploadFile` (file upload RPC, 50 MiB limit); terminal drop uses desktop host bridge (`webUtils.getPathForFile`). [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G11 -> F-C85: Files/explorer: terminal local file links open file tabs (main or side); explorer sidebar is a pane host (`explorer`) that also accepts terminal tabs; mobile file explorer drawer. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G11 -> F-F4: Files/explorer: terminal local file links open file tabs (main or side); explorer sidebar is a pane host (`explorer`) that also accepts terminal tabs; mobile file explorer drawer. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G11 -> F-H29: Files/explorer: terminal local file links open file tabs (main or side); explorer sidebar is a pane host (`explorer`) that also accepts terminal tabs; mobile file explorer drawer. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G11 -> F-K10: Files/explorer: terminal local file links open file tabs (main or side); explorer sidebar is a pane host (`explorer`) that also accepts terminal tabs; mobile file explorer drawer. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G14 -> F-A1: Settings: `terminalScrollbackLines`, `monoFontFamily`, `codeFontSize`, `serviceUrlBehavior`, host terminal profiles, host agent-hooks and browser-tools switches, Browser data clear, keyboard shortcut registry (`keyboard/keyboard-shortcuts.ts`, `terminal:false` contexts). [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G14 -> F-A83: Settings: `terminalScrollbackLines`, `monoFontFamily`, `codeFontSize`, `serviceUrlBehavior`, host terminal profiles, host agent-hooks and browser-tools switches, Browser data clear, keyboard shortcut registry (`keyboard/keyboard-shortcuts.ts`, `terminal:false` contexts). [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G14 -> F-H29: Settings: `terminalScrollbackLines`, `monoFontFamily`, `codeFontSize`, `serviceUrlBehavior`, host terminal profiles, host agent-hooks and browser-tools switches, Browser data clear, keyboard shortcut registry (`keyboard/keyboard-shortcuts.ts`, `terminal:false` contexts). [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G14 -> F-K31: Settings: `terminalScrollbackLines`, `monoFontFamily`, `codeFontSize`, `serviceUrlBehavior`, host terminal profiles, host agent-hooks and browser-tools switches, Browser data clear, keyboard shortcut registry (`keyboard/keyboard-shortcuts.ts`, `terminal:false` contexts). [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G14 -> F-L22: Settings: `terminalScrollbackLines`, `monoFontFamily`, `codeFontSize`, `serviceUrlBehavior`, host terminal profiles, host agent-hooks and browser-tools switches, Browser data clear, keyboard shortcut registry (`keyboard/keyboard-shortcuts.ts`, `terminal:false` contexts). [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G32 -> F-A101: Providers/models: terminal profiles reuse provider icon names (`provider-icon-names.ts`); OpenCode helper processes use the managed-process ledger. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G6 -> F-L7: CLI: `paseo terminal *`, `paseo script *`, `paseo hooks` (activity reporter). [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G10 -> F-H19: Relay/transport: terminal binary frames and snapshots over the multiplexed relay (no bufferedAmount signal -> byte-threshold fallback); `terminal_chunked_replay` capability negotiation. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G18 -> F-C86: Plugins: plugin panels are another tab kind in the same layout system; plugin panels/workspace targets share `WorkspaceTabTarget`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G18 -> F-M21: Plugins: plugin panels are another tab kind in the same layout system; plugin panels/workspace targets share `WorkspaceTabTarget`. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G17 -> F-A79: Command center/launcher: new-tab launcher builds terminal/profile/browser entries; `workspace-terminal-new` and tab shortcuts. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G17 -> F-C90: Command center/launcher: new-tab launcher builds terminal/profile/browser entries; `workspace-terminal-new` and tab shortcuts. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-G17 -> F-K40: Command center/launcher: new-tab launcher builds terminal/profile/browser entries; `workspace-terminal-new` and tab shortcuts. [Survey: area-4-terminals-scripts-browser.md#cross-area-connections]

F-H36 -> F-A10: Agents/sessions: every session starts with `hello`/admission (this area) and receives `server_info`; restart recovery (`agents.resumeOnStartup`, `createRestartRecovery`) resumes agents after daemon restart/update; connected hosts let agent MCP tools (`list_hosts`, `serverId` on `create_agent`, `send_agent_prompt`, `get_agent_status`, `get_agent_activity`, `list_agents`, `create_workspace`, `list_workspaces`) act on remote daemons; `agents.providers` config and `provider_diagnostic_*` feed the providers area. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H36 -> F-A100: Agents/sessions: every session starts with `hello`/admission (this area) and receives `server_info`; restart recovery (`agents.resumeOnStartup`, `createRestartRecovery`) resumes agents after daemon restart/update; connected hosts let agent MCP tools (`list_hosts`, `serverId` on `create_agent`, `send_agent_prompt`, `get_agent_status`, `get_agent_activity`, `list_agents`, `create_workspace`, `list_workspaces`) act on remote daemons; `agents.providers` config and `provider_diagnostic_*` feed the providers area. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H36 -> F-C68: Agents/sessions: every session starts with `hello`/admission (this area) and receives `server_info`; restart recovery (`agents.resumeOnStartup`, `createRestartRecovery`) resumes agents after daemon restart/update; connected hosts let agent MCP tools (`list_hosts`, `serverId` on `create_agent`, `send_agent_prompt`, `get_agent_status`, `get_agent_activity`, `list_agents`, `create_workspace`, `list_workspaces`) act on remote daemons; `agents.providers` config and `provider_diagnostic_*` feed the providers area. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H36 -> F-I12: Agents/sessions: every session starts with `hello`/admission (this area) and receives `server_info`; restart recovery (`agents.resumeOnStartup`, `createRestartRecovery`) resumes agents after daemon restart/update; connected hosts let agent MCP tools (`list_hosts`, `serverId` on `create_agent`, `send_agent_prompt`, `get_agent_status`, `get_agent_activity`, `list_agents`, `create_workspace`, `list_workspaces`) act on remote daemons; `agents.providers` config and `provider_diagnostic_*` feed the providers area. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H36 -> F-L22: Agents/sessions: every session starts with `hello`/admission (this area) and receives `server_info`; restart recovery (`agents.resumeOnStartup`, `createRestartRecovery`) resumes agents after daemon restart/update; connected hosts let agent MCP tools (`list_hosts`, `serverId` on `create_agent`, `send_agent_prompt`, `get_agent_status`, `get_agent_activity`, `list_agents`, `create_workspace`, `list_workspaces`) act on remote daemons; `agents.providers` config and `provider_diagnostic_*` feed the providers area. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H55 -> F-A3: Workspaces/projects: per-host directory sync and replica cache (`A/runtime/directory-sync`, `replica-cache`) hydrate agents/workspaces on connect; `workspaces`, `projects` host-settings sections live under the host switcher; onboarding "projects" step and `paseo [path]` / `--open-project` launches create projects/workspaces. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H55 -> F-C98: Workspaces/projects: per-host directory sync and replica cache (`A/runtime/directory-sync`, `replica-cache`) hydrate agents/workspaces on connect; `workspaces`, `projects` host-settings sections live under the host switcher; onboarding "projects" step and `paseo [path]` / `--open-project` launches create projects/workspaces. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H55 -> F-L79: Workspaces/projects: per-host directory sync and replica cache (`A/runtime/directory-sync`, `replica-cache`) hydrate agents/workspaces on connect; `workspaces`, `projects` host-settings sections live under the host switcher; onboarding "projects" step and `paseo [path]` / `--open-project` launches create projects/workspaces. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H1 -> F-G10: Terminals: restart/stop/update flows decide whether the terminal worker survives (`keepTerminals`, `killTerminals`, `terminalCleanupUnconfirmed`, `terminalsUnavailable`). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H49 -> F-A3: Permissions/questions/notifications/attention: `client_heartbeat` (focus, visibility, activity) and push token registration (`registerPushToken`, push area) ride the same session; desktop native notifications (`D/features/notifications.ts`) and `playSound` setting; `paseo://h/<serverId>/agent/<agentId>` deep links open an agent from outside (notification clicks and agent links). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H49 -> F-A262: Permissions/questions/notifications/attention: `client_heartbeat` (focus, visibility, activity) and push token registration (`registerPushToken`, push area) ride the same session; desktop native notifications (`D/features/notifications.ts`) and `playSound` setting; `paseo://h/<serverId>/agent/<agentId>` deep links open an agent from outside (notification clicks and agent links). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H49 -> F-I3: Permissions/questions/notifications/attention: `client_heartbeat` (focus, visibility, activity) and push token registration (`registerPushToken`, push area) ride the same session; desktop native notifications (`D/features/notifications.ts`) and `playSound` setting; `paseo://h/<serverId>/agent/<agentId>` deep links open an agent from outside (notification clicks and agent links). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H49 -> F-J30: Permissions/questions/notifications/attention: `client_heartbeat` (focus, visibility, activity) and push token registration (`registerPushToken`, push area) ride the same session; desktop native notifications (`D/features/notifications.ts`) and `playSound` setting; `paseo://h/<serverId>/agent/<agentId>` deep links open an agent from outside (notification clicks and agent links). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H49 -> F-K1: Permissions/questions/notifications/attention: `client_heartbeat` (focus, visibility, activity) and push token registration (`registerPushToken`, push area) ride the same session; desktop native notifications (`D/features/notifications.ts`) and `playSound` setting; `paseo://h/<serverId>/agent/<agentId>` deep links open an agent from outside (notification clicks and agent links). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H49 -> F-L14: Permissions/questions/notifications/attention: `client_heartbeat` (focus, visibility, activity) and push token registration (`registerPushToken`, push area) ride the same session; desktop native notifications (`D/features/notifications.ts`) and `playSound` setting; `paseo://h/<serverId>/agent/<agentId>` deep links open an agent from outside (notification clicks and agent links). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H45 -> F-B69: Attachments: desktop attachment storage IPC (`write_attachment_*`, `copy_attachment_file`, `read_file_base64`, GC) in `daemon-manager.ts`. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H49 -> F-C91: Browser/editor features hosted by the desktop shell: workspace browser webviews, browser automation (`browser.host.register.request` capability `CLIENT_CAPS.browserHost` advertised in hello when `paseoDesktop.browser` exists, `A/runtime/host-runtime.ts` createDefaultDeps), editor targets (open in VS Code, etc.). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H49 -> F-F14: Browser/editor features hosted by the desktop shell: workspace browser webviews, browser automation (`browser.host.register.request` capability `CLIENT_CAPS.browserHost` advertised in hello when `paseoDesktop.browser` exists, `A/runtime/host-runtime.ts` createDefaultDeps), editor targets (open in VS Code, etc.). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H49 -> F-G36: Browser/editor features hosted by the desktop shell: workspace browser webviews, browser automation (`browser.host.register.request` capability `CLIENT_CAPS.browserHost` advertised in hello when `paseoDesktop.browser` exists, `A/runtime/host-runtime.ts` createDefaultDeps), editor targets (open in VS Code, etc.). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H9 -> F-B90: Settings: daemon `config.json` (voice/dictation, MCP injection `mcp.injectIntoAgents`, `appendSystemPrompt`, `terminalProfiles`, `agentProfiles`, `autoArchiveAfterMerge`, `enableTerminalAgentHooks`, `idleRuntimeCleanup`, `resumeOnStartup`) edited through `set_daemon_config_request` from host-settings pages; per-device app settings `@paseo:app-settings` (+ `@paseo:settings-migrations`) and `desktop-settings.json` are not synced across hosts. / Voice: `features.dictation/voiceMode` config keys and Settings > Diagnostics "Test audio" (voice area). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H9 -> F-G26: Settings: daemon `config.json` (voice/dictation, MCP injection `mcp.injectIntoAgents`, `appendSystemPrompt`, `terminalProfiles`, `agentProfiles`, `autoArchiveAfterMerge`, `enableTerminalAgentHooks`, `idleRuntimeCleanup`, `resumeOnStartup`) edited through `set_daemon_config_request` from host-settings pages; per-device app settings `@paseo:app-settings` (+ `@paseo:settings-migrations`) and `desktop-settings.json` are not synced across hosts. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H9 -> F-I11: Settings: daemon `config.json` (voice/dictation, MCP injection `mcp.injectIntoAgents`, `appendSystemPrompt`, `terminalProfiles`, `agentProfiles`, `autoArchiveAfterMerge`, `enableTerminalAgentHooks`, `idleRuntimeCleanup`, `resumeOnStartup`) edited through `set_daemon_config_request` from host-settings pages; per-device app settings `@paseo:app-settings` (+ `@paseo:settings-migrations`) and `desktop-settings.json` are not synced across hosts. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H9 -> F-L19: Settings: daemon `config.json` (voice/dictation, MCP injection `mcp.injectIntoAgents`, `appendSystemPrompt`, `terminalProfiles`, `agentProfiles`, `autoArchiveAfterMerge`, `enableTerminalAgentHooks`, `idleRuntimeCleanup`, `resumeOnStartup`) edited through `set_daemon_config_request` from host-settings pages; per-device app settings `@paseo:app-settings` (+ `@paseo:settings-migrations`) and `desktop-settings.json` are not synced across hosts. / Plugins: `plugin.catalog` sync per host (`PluginCatalogSync` mounted per host in `A/app/_layout.tsx:165`), `pluginsEnabled`/`plugins` config keys, plugin clients connect with `clientType`/ephemeral-plugin lifecycle sessions. / Voice: `features.dictation/voiceMode` config keys and Settings > Diagnostics "Test audio" (voice area). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H9 -> F-A2: Plugins: `plugin.catalog` sync per host (`PluginCatalogSync` mounted per host in `A/app/_layout.tsx:165`), `pluginsEnabled`/`plugins` config keys, plugin clients connect with `clientType`/ephemeral-plugin lifecycle sessions. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H9 -> F-M21: Plugins: `plugin.catalog` sync per host (`PluginCatalogSync` mounted per host in `A/app/_layout.tsx:165`), `pluginsEnabled`/`plugins` config keys, plugin clients connect with `clientType`/ephemeral-plugin lifecycle sessions. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H59 -> F-A264: Hub: daemon-to-Hub relationship RPCs (`hub.management.daemon.connect|get_status|disconnect|permissions.update`) and `hub.execute` permission (CLI `paseo hub ...`; no app UI found). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H59 -> F-N2: Hub: daemon-to-Hub relationship RPCs (`hub.management.daemon.connect|get_status|disconnect|permissions.update`) and `hub.execute` permission (CLI `paseo hub ...`; no app UI found). [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H59 -> F-G29: Service proxy / web UI: `daemon.serviceProxy`, `features.webUi` and the Host-header/CORS rules defined in this area gate those features. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-H54 -> F-L76: i18n: all strings are `t()` keys under `pairing.*`, `settings.host.*`, `settings.about.*`, `settings.diagnostics.*`, `desktop.*`, `onboarding.*`, `changelog.*`, `startup.*` in `A/i18n/resources/{en,ko,ja,fr,es,pt-BR,ru,zh-CN,ar}.ts`. [Survey: area-5-hosts-devices-desktop.md#cross-area-connections]

F-I16 -> F-A6: Schedules -> agents: `new-agent` runs call `createAgent` (kind `mcp`, labels `paseo.schedule-id`/`paseo.schedule-run`, `unattended:true`) and `runAgent`; heartbeats call `startAgentRun` with steer on an existing agent and use the system-notification prompt wrapper (`formatSystemNotificationPrompt`); agent archive (`setAgentArchivedCallback`) completes schedules; permission requests make a run fail. [Survey: .area6-parts/A.md#cross-area-connections]

F-I16 -> F-A260: Schedules -> agents: `new-agent` runs call `createAgent` (kind `mcp`, labels `paseo.schedule-id`/`paseo.schedule-run`, `unattended:true`) and `runAgent`; heartbeats call `startAgentRun` with steer on an existing agent and use the system-notification prompt wrapper (`formatSystemNotificationPrompt`); agent archive (`setAgentArchivedCallback`) completes schedules; permission requests make a run fail. [Survey: .area6-parts/A.md#cross-area-connections]

F-I16 -> F-B26: Schedules -> agents: `new-agent` runs call `createAgent` (kind `mcp`, labels `paseo.schedule-id`/`paseo.schedule-run`, `unattended:true`) and `runAgent`; heartbeats call `startAgentRun` with steer on an existing agent and use the system-notification prompt wrapper (`formatSystemNotificationPrompt`); agent archive (`setAgentArchivedCallback`) completes schedules; permission requests make a run fail. [Survey: .area6-parts/A.md#cross-area-connections]

F-I16 -> F-C52: Schedules -> agents: `new-agent` runs call `createAgent` (kind `mcp`, labels `paseo.schedule-id`/`paseo.schedule-run`, `unattended:true`) and `runAgent`; heartbeats call `startAgentRun` with steer on an existing agent and use the system-notification prompt wrapper (`formatSystemNotificationPrompt`); agent archive (`setAgentArchivedCallback`) completes schedules; permission requests make a run fail. [Survey: .area6-parts/A.md#cross-area-connections]

F-I16 -> F-J12: Schedules -> agents: `new-agent` runs call `createAgent` (kind `mcp`, labels `paseo.schedule-id`/`paseo.schedule-run`, `unattended:true`) and `runAgent`; heartbeats call `startAgentRun` with steer on an existing agent and use the system-notification prompt wrapper (`formatSystemNotificationPrompt`); agent archive (`setAgentArchivedCallback`) completes schedules; permission requests make a run fail. [Survey: .area6-parts/A.md#cross-area-connections]

F-I16 -> F-A3: Schedules -> workspaces: per-run workspace via `createDirectoryWorkspace` / `createPaseoWorktreeWorkspace`; archived after the run unless `archiveOnFinish===false` (new-agent only); recovery after daemon restart archives interrupted run workspaces. [Survey: .area6-parts/A.md#cross-area-connections]

F-I16 -> F-C49: Schedules -> workspaces: per-run workspace via `createDirectoryWorkspace` / `createPaseoWorktreeWorkspace`; archived after the run unless `archiveOnFinish===false` (new-agent only); recovery after daemon restart archives interrupted run workspaces. [Survey: .area6-parts/A.md#cross-area-connections]

F-I16 -> F-H59: Schedules -> workspaces: per-run workspace via `createDirectoryWorkspace` / `createPaseoWorktreeWorkspace`; archived after the run unless `archiveOnFinish===false` (new-agent only); recovery after daemon restart archives interrupted run workspaces. [Survey: .area6-parts/A.md#cross-area-connections]

F-I17 -> F-A3: Schedules -> providers/models: form model reuses create-agent provider snapshot/model catalog and form preferences (`create-agent-preferences`, `provider-selection`); `thinkingOptionId`, `modeId`, `featureValues` stored in target config. [Survey: .area6-parts/A.md#cross-area-connections]

F-I17 -> F-A100: Schedules -> providers/models: form model reuses create-agent provider snapshot/model catalog and form preferences (`create-agent-preferences`, `provider-selection`); `thinkingOptionId`, `modeId`, `featureValues` stored in target config. [Survey: .area6-parts/A.md#cross-area-connections]

F-I17 -> F-L19: Schedules -> providers/models: form model reuses create-agent provider snapshot/model catalog and form preferences (`create-agent-preferences`, `provider-selection`); `thinkingOptionId`, `modeId`, `featureValues` stored in target config. [Survey: .area6-parts/A.md#cross-area-connections]

F-I5 -> F-C97: Schedules -> projects/hosts: project targets from `useProjects`, host list from `useHosts`/host runtime; aggregated across hosts client-side (no server aggregation); invalidated on host reconnect. [Survey: .area6-parts/A.md#cross-area-connections]

F-I5 -> F-H29: Schedules -> projects/hosts: project targets from `useProjects`, host list from `useHosts`/host runtime; aggregated across hosts client-side (no server aggregation); invalidated on host reconnect. [Survey: .area6-parts/A.md#cross-area-connections]

F-I9 -> F-A3: MCP `create_schedule`/`create_heartbeat`/etc. share `ScheduleService` (`createOrReplace` idempotency) with the app and CLI; CLI/MCP list hide heartbeats (`new-agent` filter) while the app list shows both. [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-A6: MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`). [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-A111: MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`). [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-A263: MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`). [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-B90: MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`). [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-C43: MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`). [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-G31: MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`). [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-H36: MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`). [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-L22: MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`). / Settings: `config.mcp.injectIntoAgents`, `config.browserTools.enabled` (BrowserToolsOptInCard `screens/settings/browser-tools-card.tsx:15`), `providers.<id>.paseoTools`, `agents.skills.selection`. [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-M21: MCP tools -> agent manager (`createAgentCommand`, `agentMessages().enqueue`, `finishSubscriptions().register` for notifyOnFinish callbacks), permissions (`list_pending_permissions`, `respond_to_permission`), terminals (`terminalManager`), workspace scripts service, workspace registry/archive service, provider snapshot manager (`list_providers/list_models/inspect_provider`), daemon config (`list_profiles` reads `agentProfiles`; policy from `providers.<id>.paseoTools`), connected hosts (`list_hosts`, `serverId` fan-out via `DaemonClient`), browser automation broker (browser tools -> desktop browser host), plugins (plugin-declared tools appended; `plugin-tools.ts`), voice (`speak`). [Survey: .area6-parts/A.md#cross-area-connections]

F-I11 -> F-A10: MCP injection -> every provider's launch config (`mcpServers.paseo`), env `PASEO_AGENT_ID`/`PASEO_AGENT_CWD`, daemon password auth (`/mcp/agents` self-authenticating), OpenCode bridge manifest, omp native host tools. [Survey: .area6-parts/A.md#cross-area-connections]

F-I11 -> F-A79: MCP injection -> every provider's launch config (`mcpServers.paseo`), env `PASEO_AGENT_ID`/`PASEO_AGENT_CWD`, daemon password auth (`/mcp/agents` self-authenticating), OpenCode bridge manifest, omp native host tools. [Survey: .area6-parts/A.md#cross-area-connections]

F-I11 -> F-H59: MCP injection -> every provider's launch config (`mcpServers.paseo`), env `PASEO_AGENT_ID`/`PASEO_AGENT_CWD`, daemon password auth (`/mcp/agents` self-authenticating), OpenCode bridge manifest, omp native host tools. [Survey: .area6-parts/A.md#cross-area-connections]

F-I11 -> F-L19: MCP injection -> every provider's launch config (`mcpServers.paseo`), env `PASEO_AGENT_ID`/`PASEO_AGENT_CWD`, daemon password auth (`/mcp/agents` self-authenticating), OpenCode bridge manifest, omp native host tools. [Survey: .area6-parts/A.md#cross-area-connections]

F-I19 -> F-A1: Orchestration skills -> daemon config (`agents.skills.selection`), filesystem homes of Claude/Codex/agents-compatible CLIs, onboarding, settings host page, desktop legacy store. [Survey: .area6-parts/A.md#cross-area-connections]

F-I19 -> F-H45: Orchestration skills -> daemon config (`agents.skills.selection`), filesystem homes of Claude/Codex/agents-compatible CLIs, onboarding, settings host page, desktop legacy store. [Survey: .area6-parts/A.md#cross-area-connections]

F-I19 -> F-L19: Orchestration skills -> daemon config (`agents.skills.selection`), filesystem homes of Claude/Codex/agents-compatible CLIs, onboarding, settings host page, desktop legacy store. [Survey: .area6-parts/A.md#cross-area-connections]

F-I11 -> F-D14: Heartbeat CLI/MCP -> caller identity from env `PASEO_AGENT_ID` / `callerAgentId` query param. [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-A1: Settings: `config.mcp.injectIntoAgents`, `config.browserTools.enabled` (BrowserToolsOptInCard `screens/settings/browser-tools-card.tsx:15`), `providers.<id>.paseoTools`, `agents.skills.selection`. [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-A100: Settings: `config.mcp.injectIntoAgents`, `config.browserTools.enabled` (BrowserToolsOptInCard `screens/settings/browser-tools-card.tsx:15`), `providers.<id>.paseoTools`, `agents.skills.selection`. [Survey: .area6-parts/A.md#cross-area-connections]

F-I18 -> F-G36: Settings: `config.mcp.injectIntoAgents`, `config.browserTools.enabled` (BrowserToolsOptInCard `screens/settings/browser-tools-card.tsx:15`), `providers.<id>.paseoTools`, `agents.skills.selection`. [Survey: .area6-parts/A.md#cross-area-connections]

F-J36 -> F-A2: Agent lifecycle and pending permissions drive attention, sidebar status and notification routing; stored unread state and notification callbacks have separate boundaries (corrections in ../gaps.md). Parent finish subscriptions are agent delivery rather than OS notifications. [Survey: .area6-parts/B.md#cross-area-connections]

F-J36 -> F-A262: Agent lifecycle and pending permissions drive attention, sidebar status and notification routing; stored unread state and notification callbacks have separate boundaries (corrections in ../gaps.md). Parent finish subscriptions are agent delivery rather than OS notifications. [Survey: .area6-parts/B.md#cross-area-connections]

F-J36 -> F-K10: Agent lifecycle and pending permissions drive attention, sidebar status and notification routing; stored unread state and notification callbacks have separate boundaries (corrections in ../gaps.md). Parent finish subscriptions are agent delivery rather than OS notifications. [Survey: .area6-parts/B.md#cross-area-connections]

F-J7 -> F-H18: Client heartbeat visibility/focus/activity connects websocket presence, notification suppression, recipient selection and push-token lease renewal. [Survey: .area6-parts/B.md#cross-area-connections]

F-J7 -> F-I19: Client heartbeat visibility/focus/activity connects websocket presence, notification suppression, recipient selection and push-token lease renewal. [Survey: .area6-parts/B.md#cross-area-connections]

F-J7 -> F-K1: Client heartbeat visibility/focus/activity connects websocket presence, notification suppression, recipient selection and push-token lease renewal. [Survey: .area6-parts/B.md#cross-area-connections]

F-J12 -> F-A3: Notification taps route host/workspace/agent/terminal identity through connection bootstrap and pane navigation; Electron/web permissions and Expo/F-Droid capabilities determine available display channels. [Survey: .area6-parts/B.md#cross-area-connections]

F-J12 -> F-A269: Notification taps route host/workspace/agent/terminal identity through connection bootstrap and pane navigation; Electron/web permissions and Expo/F-Droid capabilities determine available display channels. [Survey: .area6-parts/B.md#cross-area-connections]

F-J12 -> F-C90: Notification taps route host/workspace/agent/terminal identity through connection bootstrap and pane navigation; Electron/web permissions and Expo/F-Droid capabilities determine available display channels. [Survey: .area6-parts/B.md#cross-area-connections]

F-J12 -> F-G10: Notification taps route host/workspace/agent/terminal identity through connection bootstrap and pane navigation; Electron/web permissions and Expo/F-Droid capabilities determine available display channels. [Survey: .area6-parts/B.md#cross-area-connections]

F-J12 -> F-H34: Notification taps route host/workspace/agent/terminal identity through connection bootstrap and pane navigation; Electron/web permissions and Expo/F-Droid capabilities determine available display channels. [Survey: .area6-parts/B.md#cross-area-connections]

F-J12 -> F-K29: Notification taps route host/workspace/agent/terminal identity through connection bootstrap and pane navigation; Electron/web permissions and Expo/F-Droid capabilities determine available display channels. [Survey: .area6-parts/B.md#cross-area-connections]

F-J46 -> F-B42: Unread state feeds sidebar/tab/favicons/dock badge; composer focus/send, pane blur and workspace read/unread actions clear or restore eligible attention. [Survey: .area6-parts/B.md#cross-area-connections]

F-J46 -> F-C90: Unread state feeds sidebar/tab/favicons/dock badge; composer focus/send, pane blur and workspace read/unread actions clear or restore eligible attention. [Survey: .area6-parts/B.md#cross-area-connections]

F-J46 -> F-K10: Unread state feeds sidebar/tab/favicons/dock badge; composer focus/send, pane blur and workspace read/unread actions clear or restore eligible attention. [Survey: .area6-parts/B.md#cross-area-connections]

F-J80 -> F-A1: Usage source plugins map provider credentials/accounts to daemon queries, sidebar pinned windows, host-filtered modal and per-agent context details. Account quota and model-context usage remain distinct. [Survey: .area6-parts/B.md#cross-area-connections]

F-J80 -> F-A67: Usage source plugins map provider credentials/accounts to daemon queries, sidebar pinned windows, host-filtered modal and per-agent context details. Account quota and model-context usage remain distinct. [Survey: .area6-parts/B.md#cross-area-connections]

F-J80 -> F-H59: Usage source plugins map provider credentials/accounts to daemon queries, sidebar pinned windows, host-filtered modal and per-agent context details. Account quota and model-context usage remain distinct. [Survey: .area6-parts/B.md#cross-area-connections]

F-J80 -> F-K10: Usage source plugins map provider credentials/accounts to daemon queries, sidebar pinned windows, host-filtered modal and per-agent context details. Account quota and model-context usage remain distinct. [Survey: .area6-parts/B.md#cross-area-connections]

F-J80 -> F-M21: Usage source plugins map provider credentials/accounts to daemon queries, sidebar pinned windows, host-filtered modal and per-agent context details. Account quota and model-context usage remain distinct. [Survey: .area6-parts/B.md#cross-area-connections]

F-J57 -> F-H59: Local settings own usage preferences; Electron settings own notification sound. CLI usage and app usage share daemon reports, plugin RPCs, formatting and cache. [Survey: .area6-parts/B.md#cross-area-connections]

F-J57 -> F-L18: Local settings own usage preferences; Electron settings own notification sound. CLI usage and app usage share daemon reports, plugin RPCs, formatting and cache. [Survey: .area6-parts/B.md#cross-area-connections]

F-J57 -> F-M20: Local settings own usage preferences; Electron settings own notification sound. CLI usage and app usage share daemon reports, plugin RPCs, formatting and cache. [Survey: .area6-parts/B.md#cross-area-connections]

F-K12 -> F-A2: Sidebar rows join host/project/workspace/agent directory state with git/PR summaries, labels, scripts/terminals and attention buckets; grouping/filtering/sorting/pinning affect presentation. [Survey: .area6-parts/C.md#cross-area-connections]

F-K12 -> F-C52: Sidebar rows join host/project/workspace/agent directory state with git/PR summaries, labels, scripts/terminals and attention buckets; grouping/filtering/sorting/pinning affect presentation. [Survey: .area6-parts/C.md#cross-area-connections]

F-K12 -> F-D8: Sidebar rows join host/project/workspace/agent directory state with git/PR summaries, labels, scripts/terminals and attention buckets; grouping/filtering/sorting/pinning affect presentation. [Survey: .area6-parts/C.md#cross-area-connections]

F-K12 -> F-E14: Sidebar rows join host/project/workspace/agent directory state with git/PR summaries, labels, scripts/terminals and attention buckets; grouping/filtering/sorting/pinning affect presentation. [Survey: .area6-parts/C.md#cross-area-connections]

F-K12 -> F-G31: Sidebar rows join host/project/workspace/agent directory state with git/PR summaries, labels, scripts/terminals and attention buckets; grouping/filtering/sorting/pinning affect presentation. [Survey: .area6-parts/C.md#cross-area-connections]

F-K12 -> F-H29: Sidebar rows join host/project/workspace/agent directory state with git/PR summaries, labels, scripts/terminals and attention buckets; grouping/filtering/sorting/pinning affect presentation. [Survey: .area6-parts/C.md#cross-area-connections]

F-K12 -> F-J36: Sidebar rows join host/project/workspace/agent directory state with git/PR summaries, labels, scripts/terminals and attention buckets; grouping/filtering/sorting/pinning affect presentation. [Survey: .area6-parts/C.md#cross-area-connections]

F-K29 -> F-C70: Routes connect sidebar selection, host bootstrap, new workspace launch, split panes, mobile navigation and last-selection persistence. [Survey: .area6-parts/C.md#cross-area-connections]

F-K29 -> F-H29: Routes connect sidebar selection, host bootstrap, new workspace launch, split panes, mobile navigation and last-selection persistence. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-B42: Keyboard scopes/contexts connect shortcuts to composer editing/send, terminal, file/diff/chat find, workspace tabs, sidebar rows and command center; overrides preserve reserved/native editing behavior. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-C31: Keyboard scopes/contexts connect shortcuts to composer editing/send, terminal, file/diff/chat find, workspace tabs, sidebar rows and command center; overrides preserve reserved/native editing behavior. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-D22: Keyboard scopes/contexts connect shortcuts to composer editing/send, terminal, file/diff/chat find, workspace tabs, sidebar rows and command center; overrides preserve reserved/native editing behavior. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-F13: Keyboard scopes/contexts connect shortcuts to composer editing/send, terminal, file/diff/chat find, workspace tabs, sidebar rows and command center; overrides preserve reserved/native editing behavior. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-G11: Keyboard scopes/contexts connect shortcuts to composer editing/send, terminal, file/diff/chat find, workspace tabs, sidebar rows and command center; overrides preserve reserved/native editing behavior. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-H62: Keyboard scopes/contexts connect shortcuts to composer editing/send, terminal, file/diff/chat find, workspace tabs, sidebar rows and command center; overrides preserve reserved/native editing behavior. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-A3: Command center consumes common host/workspace/project/agent state, add-project flow, labels and action registrations; plugin contributions extend it. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-C52: Command center consumes common host/workspace/project/agent state, add-project flow, labels and action registrations; plugin contributions extend it. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-H34: Command center consumes common host/workspace/project/agent state, add-project flow, labels and action registrations; plugin contributions extend it. [Survey: .area6-parts/C.md#cross-area-connections]

F-K40 -> F-M20: Command center consumes common host/workspace/project/agent state, add-project flow, labels and action registrations; plugin contributions extend it. [Survey: .area6-parts/C.md#cross-area-connections]

F-K25 -> F-A260: Callouts and attention badges link to setup/changelog/host permission/settings/recovery screens; app settings store sidebar visibility/order and shortcut overrides. [Survey: .area6-parts/C.md#cross-area-connections]

F-K25 -> F-H29: Callouts and attention badges link to setup/changelog/host permission/settings/recovery screens; app settings store sidebar visibility/order and shortcut overrides. [Survey: .area6-parts/C.md#cross-area-connections]

F-K25 -> F-J46: Callouts and attention badges link to setup/changelog/host permission/settings/recovery screens; app settings store sidebar visibility/order and shortcut overrides. [Survey: .area6-parts/C.md#cross-area-connections]

F-K25 -> F-L24: Callouts and attention badges link to setup/changelog/host permission/settings/recovery screens; app settings store sidebar visibility/order and shortcut overrides. [Survey: .area6-parts/C.md#cross-area-connections]

F-K1 -> F-F1: The body contains no appearance implementation despite another file referring to it; complete theme/font/syntax/localization contracts are in ../gaps.md. [Survey: .area6-parts/C.md#cross-area-connections]

F-K1 -> F-L11: The body contains no appearance implementation despite another file referring to it; complete theme/font/syntax/localization contracts are in ../gaps.md. [Survey: .area6-parts/C.md#cross-area-connections]

F-L22 -> F-A9: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-L22 -> F-A100: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-L22 -> F-B90: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-L22 -> F-C70: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-L22 -> F-G29: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-L22 -> F-H19: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-L22 -> F-I11: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-L22 -> F-J18: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-L22 -> F-K10: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-L22 -> F-M21: Daemon config keys drive agents (appendSystemPrompt, mcp injection, browser tools, resume/idle cleanup), workspaces (auto-archive, terminal hooks), providers/models (providers, metadata generation), voice (features), plugins (pluginsEnabled), terminals (profiles). App settings feed chat rendering, composer send behaviour, sidebar, appearance, notifications (desktop playSound), usage preferences. Host store links settings to connection/relay/pairing. Project settings link to workspace setup scripts and service proxy. [Survey: .area6-parts/D.md#cross-area-connections]

F-M16 -> F-A3: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-M16 -> F-A190: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-M16 -> F-A100: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-M16 -> F-A260: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-M16 -> F-C6: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-M16 -> F-H36: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-M16 -> F-I12: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-M16 -> F-J80: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-M16 -> F-K40: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-M16 -> F-L72: Usage UI (part B) consumes usage sources; themes (part C) join the appearance picker; command center (part C) shows plugin items; sidebar (part C) renders plugin header/footer items; settings (part D) hosts plugin settings screens and the plugins page; MCP tools (part A) receive plugin agent tools; providers/models (other area) get plugin providers; workspace creation, agent create/open/turn/permission flows trigger hooks; timeline area renders plugin timeline items. [Survey: .area6-parts/E.md#cross-area-connections]

F-A260 -> F-A133: **Approval → provider and timeline:** request kind/actions/selectedActionId and updatedInput/permissions control tool continuation, plan implementation and question result cards; 15s answer confirmation differs from provider-turn deadlines. [Survey: gaps.md#cross-area-connections]

F-A260 -> F-A106: **Approval → provider and timeline:** request kind/actions/selectedActionId and updatedInput/permissions control tool continuation, plan implementation and question result cards; 15s answer confirmation differs from provider-turn deadlines. / **Approval → every observing client:** provider maps/snapshots, Session owned events and direct correlated reply synchronize request removal; stale/offline caches must not fabricate pending approvals. This is independent of notification recipient selection. [Survey: gaps.md#cross-area-connections]

F-A260 -> F-A10: **Approval → every observing client:** provider maps/snapshots, Session owned events and direct correlated reply synchronize request removal; stale/offline caches must not fabricate pending approvals. This is independent of notification recipient selection. [Survey: gaps.md#cross-area-connections]

F-A260 -> F-J12: **Approval → every observing client:** provider maps/snapshots, Session owned events and direct correlated reply synchronize request removal; stale/offline caches must not fabricate pending approvals. This is independent of notification recipient selection. [Survey: gaps.md#cross-area-connections]

F-A262 -> F-C64: **Attention → workspace/sidebar/notifications:** pending-input bucket, durable unread edge, parent-delegation callback suppression and presence policy are separate inputs. Heartbeats route OS/push notifications without assigning exclusive answer ownership. [Survey: gaps.md#cross-area-connections]

F-A262 -> F-I9: **Attention → workspace/sidebar/notifications:** pending-input bucket, durable unread edge, parent-delegation callback suppression and presence policy are separate inputs. Heartbeats route OS/push notifications without assigning exclusive answer ownership. [Survey: gaps.md#cross-area-connections]

F-A262 -> F-J36: **Attention → workspace/sidebar/notifications:** pending-input bucket, durable unread edge, parent-delegation callback suppression and presence policy are separate inputs. Heartbeats route OS/push notifications without assigning exclusive answer ownership. [Survey: gaps.md#cross-area-connections]

F-A262 -> F-K10: **Attention → workspace/sidebar/notifications:** pending-input bucket, durable unread edge, parent-delegation callback suppression and presence policy are separate inputs. Heartbeats route OS/push notifications without assigning exclusive answer ownership. [Survey: gaps.md#cross-area-connections]

F-A265 -> F-A74: **Profiles → creation and live config:** host shared config plus provider/model/mode/feature discovery materializes into draft or serialized live setting mutations; device preferences remember values, not a live profile binding. Partial provider failures remain observable. [Survey: gaps.md#cross-area-connections]

F-A265 -> F-B7: **Profiles → creation and live config:** host shared config plus provider/model/mode/feature discovery materializes into draft or serialized live setting mutations; device preferences remember values, not a live profile binding. Partial provider failures remain observable. [Survey: gaps.md#cross-area-connections]

F-A265 -> F-H29: **Profiles → creation and live config:** host shared config plus provider/model/mode/feature discovery materializes into draft or serialized live setting mutations; device preferences remember values, not a live profile binding. Partial provider failures remain observable. [Survey: gaps.md#cross-area-connections]

F-N2 -> F-A111: **Accounts → usage/providers/Hub:** quota source+account association is not provider selection; custom provider env/plugin routing differs from core profiles. Hub CLI credentials and daemon grants have separate ownership and revocation. [Survey: gaps.md#cross-area-connections]

F-N2 -> F-H59: **Accounts → usage/providers/Hub:** quota source+account association is not provider selection; custom provider env/plugin routing differs from core profiles. Hub CLI credentials and daemon grants have separate ownership and revocation. [Survey: gaps.md#cross-area-connections]

F-N2 -> F-J80: **Accounts → usage/providers/Hub:** quota source+account association is not provider selection; custom provider env/plugin routing differs from core profiles. Hub CLI credentials and daemon grants have separate ownership and revocation. [Survey: gaps.md#cross-area-connections]

F-N2 -> F-M20: **Accounts → usage/providers/Hub:** quota source+account association is not provider selection; custom provider env/plugin routing differs from core profiles. Hub CLI credentials and daemon grants have separate ownership and revocation. [Survey: gaps.md#cross-area-connections]

F-A268 -> F-A106: **Skills → onboarding/MCP/composer:** bundled filesystem installation/selection is host-level; provider-native skill commands enter slash autocomplete independently; profile schema has no skill list. [Survey: gaps.md#cross-area-connections]

F-A268 -> F-B42: **Skills → onboarding/MCP/composer:** bundled filesystem installation/selection is host-level; provider-native skill commands enter slash autocomplete independently; profile schema has no skill list. [Survey: gaps.md#cross-area-connections]

F-A268 -> F-H29: **Skills → onboarding/MCP/composer:** bundled filesystem installation/selection is host-level; provider-native skill commands enter slash autocomplete independently; profile schema has no skill list. [Survey: gaps.md#cross-area-connections]

F-A268 -> F-I19: **Skills → onboarding/MCP/composer:** bundled filesystem installation/selection is host-level; provider-native skill commands enter slash autocomplete independently; profile schema has no skill list. [Survey: gaps.md#cross-area-connections]

F-C234 -> F-A3: **Titles/metadata → project/git/workspace:** provisional agent title and manual tab rename differ from generated workspace title/branch; host candidate order, current selection and project style overrides also drive commit/PR generation. [Survey: gaps.md#cross-area-connections]

F-C234 -> F-C58: **Titles/metadata → project/git/workspace:** provisional agent title and manual tab rename differ from generated workspace title/branch; host candidate order, current selection and project style overrides also drive commit/PR generation. [Survey: gaps.md#cross-area-connections]

F-C234 -> F-D17: **Titles/metadata → project/git/workspace:** provisional agent title and manual tab rename differ from generated workspace title/branch; host candidate order, current selection and project style overrides also drive commit/PR generation. [Survey: gaps.md#cross-area-connections]

F-C234 -> F-E14: **Titles/metadata → project/git/workspace:** provisional agent title and manual tab rename differ from generated workspace title/branch; host candidate order, current selection and project style overrides also drive commit/PR generation. [Survey: gaps.md#cross-area-connections]

F-C234 -> F-H29: **Titles/metadata → project/git/workspace:** provisional agent title and manual tab rename differ from generated workspace title/branch; host candidate order, current selection and project style overrides also drive commit/PR generation. [Survey: gaps.md#cross-area-connections]

F-L73 -> F-D22: **Appearance → every presentation surface:** theme tokens/typographic ramp/width/syntax affect chat/editor/diff/terminal/sidebar; plugin contribution identity and host availability govern contributed themes, local settings govern selection. [Survey: gaps.md#cross-area-connections]

F-L73 -> F-F11: **Appearance → every presentation surface:** theme tokens/typographic ramp/width/syntax affect chat/editor/diff/terminal/sidebar; plugin contribution identity and host availability govern contributed themes, local settings govern selection. [Survey: gaps.md#cross-area-connections]

F-L73 -> F-G10: **Appearance → every presentation surface:** theme tokens/typographic ramp/width/syntax affect chat/editor/diff/terminal/sidebar; plugin contribution identity and host availability govern contributed themes, local settings govern selection. [Survey: gaps.md#cross-area-connections]

F-L73 -> F-H29: **Appearance → every presentation surface:** theme tokens/typographic ramp/width/syntax affect chat/editor/diff/terminal/sidebar; plugin contribution identity and host availability govern contributed themes, local settings govern selection. [Survey: gaps.md#cross-area-connections]

F-L73 -> F-K10: **Appearance → every presentation surface:** theme tokens/typographic ramp/width/syntax affect chat/editor/diff/terminal/sidebar; plugin contribution identity and host availability govern contributed themes, local settings govern selection. [Survey: gaps.md#cross-area-connections]

F-L73 -> F-M20: **Appearance → every presentation surface:** theme tokens/typographic ramp/width/syntax affect chat/editor/diff/terminal/sidebar; plugin contribution identity and host availability govern contributed themes, local settings govern selection. [Survey: gaps.md#cross-area-connections]

F-L75 -> F-M21: **Localization → settings/onboarding/plugins:** bundled synchronous locale backend renders core UI before children; plugin text resolution is separate, locale preference local to client. [Survey: gaps.md#cross-area-connections]

F-L77 -> F-C97: **Onboarding → host hydration/projects/skills/pairing:** startup gate demands directory; step visit tokens contain late async results, project add and orchestration mutations advance only after success; skipped flow does not rollback prior choices. [Survey: gaps.md#cross-area-connections]

F-L77 -> F-H24: **Onboarding → host hydration/projects/skills/pairing:** startup gate demands directory; step visit tokens contain late async results, project add and orchestration mutations advance only after success; skipped flow does not rollback prior choices. [Survey: gaps.md#cross-area-connections]

F-L77 -> F-I19: **Onboarding → host hydration/projects/skills/pairing:** startup gate demands directory; step visit tokens contain late async results, project add and orchestration mutations advance only after success; skipped flow does not rollback prior choices. [Survey: gaps.md#cross-area-connections]

F-L79 -> F-A106: **Storage → reconnect/layout/drafts:** validated local envelopes protect hydration; replica row schemas/cursors/budgets paint stale state safely, then authoritative host state replaces it; pending permissions and provider persistence handles are stripped. [Survey: gaps.md#cross-area-connections]

F-L79 -> F-C86: **Storage → reconnect/layout/drafts:** validated local envelopes protect hydration; replica row schemas/cursors/budgets paint stale state safely, then authoritative host state replaces it; pending permissions and provider persistence handles are stripped. [Survey: gaps.md#cross-area-connections]

F-L79 -> F-H29: **Storage → reconnect/layout/drafts:** validated local envelopes protect hydration; replica row schemas/cursors/budgets paint stale state safely, then authoritative host state replaces it; pending permissions and provider persistence handles are stripped. [Survey: gaps.md#cross-area-connections]

F-A268 -> F-A2: **Performance → timeline/runtime/query:** opt-in native/browser traces, explicit query invalidation, cache coalescing and startup appearance gate reduce work without changing agent/provider lifecycle semantics. [Survey: gaps.md#cross-area-connections]

F-A268 -> F-A141: **Performance → timeline/runtime/query:** opt-in native/browser traces, explicit query invalidation, cache coalescing and startup appearance gate reduce work without changing agent/provider lifecycle semantics. [Survey: gaps.md#cross-area-connections]

F-A268 -> F-A88: **Performance → timeline/runtime/query:** opt-in native/browser traces, explicit query invalidation, cache coalescing and startup appearance gate reduce work without changing agent/provider lifecycle semantics. [Survey: gaps.md#cross-area-connections]

F-A268 -> F-G36: **Performance → timeline/runtime/query:** opt-in native/browser traces, explicit query invalidation, cache coalescing and startup appearance gate reduce work without changing agent/provider lifecycle semantics. [Survey: gaps.md#cross-area-connections]

F-A268 -> F-H62: **Performance → timeline/runtime/query:** opt-in native/browser traces, explicit query invalidation, cache coalescing and startup appearance gate reduce work without changing agent/provider lifecycle semantics. [Survey: gaps.md#cross-area-connections]

F-C235 -> F-C52: **Workspace labels → compound registry and sidebar:** retained journal bounds and generation drive catch-up; catalog/workspace rollback uncertainty blocks mutations rather than publishing divergent filter state; full UI/RPC in p1. [Survey: gaps.md#cross-area-connections]

F-C235 -> F-K10: **Workspace labels → compound registry and sidebar:** retained journal bounds and generation drive catch-up; catalog/workspace rollback uncertainty blocks mutations rather than publishing divergent filter state; full UI/RPC in p1. [Survey: gaps.md#cross-area-connections]

F-A270 -> F-A6: **Subagents → parent/host/layout/lifecycle:** cross-host address joins managed children, provider-native read-only descriptors, parent navigation and archive/detach. Local provider dismissal differs from real child archive; child tab close differs from root close. [Survey: gaps.md#cross-area-connections]

F-A270 -> F-A106: **Subagents → parent/host/layout/lifecycle:** cross-host address joins managed children, provider-native read-only descriptors, parent navigation and archive/detach. Local provider dismissal differs from real child archive; child tab close differs from root close. [Survey: gaps.md#cross-area-connections]

F-A270 -> F-C86: **Subagents → parent/host/layout/lifecycle:** cross-host address joins managed children, provider-native read-only descriptors, parent navigation and archive/detach. Local provider dismissal differs from real child archive; child tab close differs from root close. [Survey: gaps.md#cross-area-connections]

F-A270 -> F-H35: **Subagents → parent/host/layout/lifecycle:** cross-host address joins managed children, provider-native read-only descriptors, parent navigation and archive/detach. Local provider dismissal differs from real child archive; child tab close differs from root close. [Survey: gaps.md#cross-area-connections]

F-A270 -> F-K29: **Subagents → parent/host/layout/lifecycle:** cross-host address joins managed children, provider-native read-only descriptors, parent navigation and archive/detach. Local provider dismissal differs from real child archive; child tab close differs from root close. [Survey: gaps.md#cross-area-connections]

F-N3 -> F-A6: **Hub → authorization/provider/worktree:** CLI login enrolls relationship separately; hub.execute gates owned execution, shared create/archive pipeline enforces ownership and exact MCP preapproval, authority generations retire stale work. [Survey: gaps.md#cross-area-connections]

F-N3 -> F-A106: **Hub → authorization/provider/worktree:** CLI login enrolls relationship separately; hub.execute gates owned execution, shared create/archive pipeline enforces ownership and exact MCP preapproval, authority generations retire stale work. [Survey: gaps.md#cross-area-connections]

F-N3 -> F-C84: **Hub → authorization/provider/worktree:** CLI login enrolls relationship separately; hub.execute gates owned execution, shared create/archive pipeline enforces ownership and exact MCP preapproval, authority generations retire stale work. [Survey: gaps.md#cross-area-connections]

F-N3 -> F-I11: **Hub → authorization/provider/worktree:** CLI login enrolls relationship separately; hub.execute gates owned execution, shared create/archive pipeline enforces ownership and exact MCP preapproval, authority generations retire stale work. [Survey: gaps.md#cross-area-connections]

F-K63 -> F-A3: **Web overlays → keyboard/accessibility/plugins:** top painted layer owns key/focus, IME stays with editing field and plugin popover Escape must not interrupt underlying agent; no server mutation involved. [Survey: gaps.md#cross-area-connections]

F-K63 -> F-K38: **Web overlays → keyboard/accessibility/plugins:** top painted layer owns key/focus, IME stays with editing field and plugin popover Escape must not interrupt underlying agent; no server mutation involved. [Survey: gaps.md#cross-area-connections]

F-K63 -> F-M21: **Web overlays → keyboard/accessibility/plugins:** top painted layer owns key/focus, IME stays with editing field and plugin popover Escape must not interrupt underlying agent; no server mutation involved. [Survey: gaps.md#cross-area-connections]

## Coverage

- Catalogue feature occurrences: 1419 (779 headings; 640 additional inventory entries).
- Shared entities: 14.
- Source Cross-area connections entries: 162; entries represented: 162; unique directed links: 486.
- Every source file is represented below. Multiple links may derive from one entry; several entries may collapse to one directed pair.

| Survey file | Source cross-area entries | Represented |
|---|---:|---:|
| parts/A-lifecycle.md | 12 | 12 |
| parts/B-providers.md | 5 | 5 |
| parts/C-timeline.md | 14 | 14 |
| parts/D-messaging.md | 6 | 6 |
| area-2-composer-input.md | 15 | 15 |
| parts/p1-projects-workspaces.md | 10 | 10 |
| parts/p2-git.md | 12 | 12 |
| parts/p3-forge-review.md | 13 | 13 |
| parts/p4-files.md | 11 | 11 |
| area-4-terminals-scripts-browser.md | 12 | 12 |
| area-5-hosts-devices-desktop.md | 12 | 12 |
| .area6-parts/A.md | 10 | 10 |
| .area6-parts/B.md | 6 | 6 |
| .area6-parts/C.md | 6 | 6 |
| .area6-parts/D.md | 1 | 1 |
| .area6-parts/E.md | 1 | 1 |
| gaps.md | 16 | 16 |

## Survey snapshot hashes

These SHA-256 values identify the exact survey text consolidated; the survey itself was not modified.

| File | SHA-256 |
|---|---|
| parts/A-lifecycle.md | 0e80ed6fab3b28e6181a135c076a265997f3223f1d6bfdfc5ae2fc57bcf3a563 |
| parts/B-providers.md | 9c752413457a0fff8c58e1c9ad39b2073a2c684933abb48c1a70f61d630779b1 |
| parts/C-timeline.md | 51ba6e9ce3f3f6a43df716edc572ea33b8451e505c5de493c3fe77af1de904b1 |
| parts/D-messaging.md | 012ee1edd6d06b97662e49e6645f603e8b5cbeb9aff0edfd33217dc3316974f2 |
| area-2-composer-input.md | 326cc8c45aa6963fafabd931f8381693c0a641d2dbb769b76e4110bf8aba4e09 |
| parts/p1-projects-workspaces.md | 94681b812fc539f1fadafdd4a5e3c4e5664c70bc77d6d49921144c711bb671a1 |
| parts/p2-git.md | 4599531bd03dbc73ee82df09347ad134d05fe339c9399c36909dd5a15cbb2307 |
| parts/p3-forge-review.md | 4cf2c066b3b2fccc43dc8d444f153fd60dab847504b2e37afa1a6c691d7af98c |
| parts/p4-files.md | 01e615f1eae481218772a5971c91765ca99f72db699ba6ffa12eface59faa0df |
| area-4-terminals-scripts-browser.md | 46a10d154104e02d92d1558c9f959b0381767a1fe03d73602b62772260f987a6 |
| area-5-hosts-devices-desktop.md | 19cb1c745f088e275eccf01aff9ae217f14168266dd8d4f72f232b0bf14048f4 |
| .area6-parts/A.md | 9aba1d751e290e044dba5f11cf795c961642b2a9e172cad8f95fc2c7f0a73f15 |
| .area6-parts/B.md | 2bf17390d2c17a3503f2fe88f3b5f3cc06a44d7358d941faf0e67103cd7101ea |
| .area6-parts/C.md | c711c74c2542c8d6e31bf5b65f4426b654f9a361062376fd25ac61a1157f3c49 |
| .area6-parts/D.md | 1863f2e047dc829f7d72bf187557675554f5d1febf877fbadcec275a5c43c96b |
| .area6-parts/E.md | b02fb43e588e29c22e8d366d2bd9a7f579707d9c9d41b82c1037048d2c698d44 |
| gaps.md | 61f166d8d533895185a54d578cdc9c4f5b6f4ab08299cabdba721820413ef0c7 |
