package com.afzal.ayshamusic

import android.content.ComponentName
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build

/** Fixed aliases only. Enable the destination before removing the old launcher. */
object AppIcons {
    data class Choice(val id:String,val title:String,val preview:Int)
    val choices=listOf(
        Choice("rose","Rose gold heart",R.drawable.icon_rose_preview),
        Choice("classic","Classic silver",R.drawable.icon_classic_preview),
        Choice("velvet","Velvet A · Music",R.drawable.icon_velvet_preview),
        Choice("cute","Cute rose",R.drawable.icon_cute_preview),
        Choice("red","Racing red",R.drawable.icon_red_preview),
        Choice("ivory","Red on ivory",R.drawable.icon_ivory_preview),
        Choice("butterflies","Red butterflies",R.drawable.icon_butterflies_preview),
        Choice("embossed","Embossed red · butterflies",R.drawable.icon_embossed_preview)
    )
    private fun alias(context:Context,id:String)=ComponentName(context,"com.afzal.ayshamusic.Launcher"+id.replaceFirstChar{it.uppercase()})
    fun current(context:Context):String {
        val pm=context.packageManager
        return choices.firstOrNull { c ->
            val state=pm.getComponentEnabledSetting(alias(context,c.id))
            state==PackageManager.COMPONENT_ENABLED_STATE_ENABLED || (state==PackageManager.COMPONENT_ENABLED_STATE_DEFAULT && c.id=="rose")
        }?.id ?: "rose"
    }
    @Synchronized fun select(context:Context,id:String) {
        require(choices.any{it.id==id}){"Unknown icon"}
        val pm=context.packageManager
        if(Build.VERSION.SDK_INT>=33){
            pm.setComponentEnabledSettings(choices.map { c -> PackageManager.ComponentEnabledSetting(alias(context,c.id),if(c.id==id) PackageManager.COMPONENT_ENABLED_STATE_ENABLED else PackageManager.COMPONENT_ENABLED_STATE_DISABLED,PackageManager.DONT_KILL_APP) })
        }else{
            pm.setComponentEnabledSetting(alias(context,id),PackageManager.COMPONENT_ENABLED_STATE_ENABLED,PackageManager.DONT_KILL_APP)
            choices.filter{it.id!=id}.forEach{pm.setComponentEnabledSetting(alias(context,it.id),PackageManager.COMPONENT_ENABLED_STATE_DISABLED,PackageManager.DONT_KILL_APP)}
        }
    }
    fun title(context:Context)=choices.first{it.id==current(context)}.title
}
