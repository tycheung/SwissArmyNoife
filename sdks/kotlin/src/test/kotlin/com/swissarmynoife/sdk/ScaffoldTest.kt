package com.swissarmynoife.sdk

import kotlin.test.Test
import kotlin.test.assertEquals

class ScaffoldTest {
    @Test
    fun sdkInfoHasName() {
        assertEquals("swissarmynoife-sdk-kotlin", SdkInfo.NAME)
    }
}
